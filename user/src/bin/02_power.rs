#![no_std]
#![no_main]

extern crate user_lib;

use user_lib::println;

#[unsafe(no_mangle)]
fn main() -> i32 {
    let mut buffer = [0u32; 10];
    let buffer_size = buffer.len();

    const BASE: u32 = 3;
    const POW_CEIL: u32 = 100_000;
    const MODULE: u32 = 10_007;

    let mut index = 0;
    buffer[index] = 1;

    for pow in 1..=POW_CEIL {
        let prev_value = buffer[index];
        let value = (prev_value * BASE) % MODULE;
        index = (index + 1) % buffer_size;
        buffer[index] = value;

        if pow % 2_000 == 0 {
            println!("{}^{}={}(MOD {})", BASE, pow, value, MODULE);
        }
    }
    println!("Test power OK!");
    0
}
