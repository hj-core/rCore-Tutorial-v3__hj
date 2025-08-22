#![no_std]
#![no_main]

use user_lib::{munmap, println};

extern crate user_lib;

const USER_SPACE_END: usize = 0x40_0000_0000 - (1 << 12);
const DATA_STRING: &str = "test_munmap3 failed if you see this line.";

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Try to unmap the whole user space. Kernel should kill this app due to page fault.");
    let addr = 0;
    let len = USER_SPACE_END;
    let _ = munmap(addr, len);

    println!("{}", DATA_STRING);
    -1
}
