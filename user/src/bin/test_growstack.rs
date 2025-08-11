#![no_std]
#![no_main]

use core::arch::asm;

use user_lib::println;

extern crate user_lib;

const PAGE_SIZE_ORDER: usize = 12;
const PAGE_SIZE_BYTES: usize = 1 << PAGE_SIZE_ORDER; // 4 KiB
const USER_SPACE_END: usize = 0x40_0000_0000 - PAGE_SIZE_BYTES;
const USER_STACK_MAX_SIZE_BYTES: usize = 8 << 20; // 8 MiB

#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("Test growing of user stack.");

    println!("Try to read an address that triggers stack grow.");
    let addr = USER_SPACE_END - 2 * PAGE_SIZE_BYTES + 2;
    let a = unsafe { { addr as *const u8 }.read_volatile() };
    println!("Ok. a={:#x}", a);

    println!("Try to write to an address that triggers stack grow and then read it.");
    let addr = USER_SPACE_END - 5 * PAGE_SIZE_BYTES + 2;
    unsafe { { addr as *mut u8 }.write_volatile(25) };
    let b = unsafe { { addr as *const u8 }.read_volatile() };
    println!("Ok. b={:#x}", b);

    println!("Try to read the maximum allowed user sp.");
    let addr = USER_SPACE_END - USER_STACK_MAX_SIZE_BYTES;
    let c = unsafe { { addr as *const u8 }.read_volatile() };
    println!("Ok. c={:#x}", c);

    println!("Try to read an address beyond allowed user sp; kernel should kill this app.");
    let addr = USER_SPACE_END - USER_STACK_MAX_SIZE_BYTES - 4;
    let _ = unsafe { { addr as *const u8 }.read_volatile() };
    println!("Not ok. You should not see this line");

    0
}

#[allow(dead_code)]
fn print_sp() {
    let mut sp: usize;
    unsafe { asm!("mv {}, sp", out(reg) sp) };
    println!("sp={:#x}", sp);
}
