#![no_std]
#![no_main]

use user_lib::{munmap, println};

extern crate user_lib;

const PAGE_SIZE_ORDER: usize = 12;
const PAGE_SIZE_BYTES: usize = 1 << PAGE_SIZE_ORDER; // 4 KiB
const USER_SPACE_END: usize = 0x40_0000_0000 - PAGE_SIZE_BYTES;
const USER_STACK_MAX_SIZE_BYTES: usize = 8 << 20; // 8 MiB

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Try to unmap the user stack. Kernel should kill this app due to page fault.");
    let len = USER_STACK_MAX_SIZE_BYTES;
    let addr = USER_SPACE_END - len;
    let result = munmap(addr, len);
    assert_eq!(result, 0, "Test failed if you see this line");
    -1
}
