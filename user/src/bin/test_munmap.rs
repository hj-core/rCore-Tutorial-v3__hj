#![no_std]
#![no_main]

use user_lib::{mmap, munmap, println};

extern crate user_lib;

const PAGE_SIZE_ORDER: usize = 12;
const PAGE_SIZE_BYTES: usize = 1 << PAGE_SIZE_ORDER; // 4 KiB
const USER_SPACE_END: usize = 0x40_0000_0000 - PAGE_SIZE_BYTES;

const PROT_READ: usize = 2;
const PROT_WRITE: usize = 4;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    // Try to unmap an area that is not in user space
    let addr = USER_SPACE_END;
    let len = 0x100;
    let result = munmap(addr, len);
    assert_eq!(result, -1, "munmap an area not in user space should fail");

    // Try to unmap an area that has not been mapped
    let addr = 0x8000_0000;
    let len = 100;
    let result = munmap(addr, len);
    assert_eq!(result, 0, "munmap an area not mapped has no effect.");

    // Try to unmap an area that is partially mapped
    let addr = 0x8000_0000;
    let len = PAGE_SIZE_BYTES * 3;
    let result = mmap(addr, len, PROT_READ | PROT_WRITE);
    assert_eq!(result, 0, "mmap should succeed.");

    let result = munmap(addr - PAGE_SIZE_BYTES, PAGE_SIZE_BYTES * 2);
    assert_eq!(result, 0, "munmap should succeed.");

    // Try to access the area that is still mapped
    for i in (addr + PAGE_SIZE_BYTES)..(addr + len) {
        unsafe { (i as *mut u8).write_volatile(i as u8) };
        let _ = unsafe { (i as *mut u8).read_volatile() };
    }

    // Try to access the unmapped area
    println!("Test munmap so far ok.");
    println!("Last test should trigger load page fault and kernel should kill this app.");
    unsafe {
        let _ = (addr as *mut u8).read_volatile();
    }

    println!("Test munmap failed if you see this line!");
    -1
}
