#![no_std]
#![no_main]

use user_lib::{mmap, println};

extern crate user_lib;

const PAGE_SIZE_ORDER: usize = 12;
const PAGE_SIZE_BYTES: usize = 1 << PAGE_SIZE_ORDER; // 4 KiB
const USER_SPACE_END: usize = 0x40_0000_0000 - PAGE_SIZE_BYTES;

const PROT_READ: usize = 2;
const PROT_WRITE: usize = 4;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    // Try to map an area that is not in user space
    let addr = USER_SPACE_END;
    let len = 1;
    let prot = PROT_READ;
    let result = mmap(addr, len, prot);
    assert_eq!(result, -1, "mmap an area not in user space should fail");

    // Try to map the user stack
    let addr = USER_SPACE_END - PAGE_SIZE_BYTES * 2;
    let len = PAGE_SIZE_BYTES - 1000;
    let prot = PROT_READ | PROT_WRITE;
    let result = mmap(addr, len, prot);
    assert_eq!(result, -1, "mmap the user stack should fail");

    // Try to map a read/write area
    let addr = 0x8000_0002;
    let len = 100;
    let prot = PROT_READ | PROT_WRITE;
    let result = mmap(addr, len, prot);
    assert_eq!(result, 0, "mmap should succeed");
    for i in addr..addr + len {
        unsafe { (i as *mut u8).write_volatile(i as u8) };
        let _ = unsafe { (i as *const u8).read_volatile() };
    }

    // Try to map the same area again
    let result = mmap(addr, len, prot);
    assert_eq!(result, -1, "mmap the same area again should fail");

    // Try to map a read-only area and write to it
    let addr = 0x8010_0000;
    let len = 100;
    let prot = PROT_READ;
    let result = mmap(addr, len, prot);
    assert_eq!(result, 0, "mmap should succeed");
    println!("Test mmap so far ok.");
    println!("Last test should trigger store page fault and kernel should kill this app.");
    unsafe { (addr as *mut u8).write_volatile(0) };

    println!("Test mmap failed if you see this line!");
    -1
}
