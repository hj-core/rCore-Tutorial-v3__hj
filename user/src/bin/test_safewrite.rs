#![no_std]
#![no_main]

extern crate user_lib;

use core::arch::asm;

use core::slice;
use user_lib::{println, write};

/// Expect:
/// string from data section
/// strinstring from stack section
/// strin
/// Test write OK!

const STACK_SIZE: usize = 0x2000;
const STACK_ALIGN: usize = 0x1000;

const STDOUT: usize = 1;
const DATA_STRING: &str = "string from data section\n";

unsafe fn r_sp() -> usize {
    let mut sp: usize;
    unsafe { asm!("mv {}, sp", out(reg) sp) };
    sp
}

unsafe fn stack_range() -> (usize, usize) {
    let sp = unsafe { r_sp() };
    // Require the sp to be in the top half of the user stack, which should
    // be fine for the current case.
    let top = (sp + STACK_ALIGN - 1) & (!(STACK_ALIGN - 1));
    (top - STACK_SIZE, top)
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    assert_eq!(
        write(STDOUT, unsafe {
            #[allow(clippy::zero_ptr)]
            slice::from_raw_parts(0x0 as *const _, 10)
        }),
        -1
    );
    let (bottom, top) = unsafe { stack_range() };
    assert_eq!(
        write(STDOUT, unsafe {
            slice::from_raw_parts((top - 5) as *const _, 10)
        }),
        -1
    );
    assert_eq!(
        write(STDOUT, unsafe {
            slice::from_raw_parts((bottom - 5) as *const _, 10)
        }),
        -1
    );

    assert_eq!(write(1234, DATA_STRING.as_bytes()), -1);

    assert_eq!(
        write(STDOUT, DATA_STRING.as_bytes()),
        DATA_STRING.len() as isize
    );
    assert_eq!(write(STDOUT, &DATA_STRING.as_bytes()[..5]), 5);

    let stack_string = "string from stack section\n";
    assert_eq!(
        write(STDOUT, stack_string.as_bytes()),
        stack_string.len() as isize
    );
    assert_eq!(write(STDOUT, &stack_string.as_bytes()[..5]), 5);
    println!("\nTest write OK!");
    0
}
