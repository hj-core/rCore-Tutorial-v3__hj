#![no_std]
#![no_main]

extern crate user_lib;

use user_lib::{println, yield_now};

#[unsafe(no_mangle)]
fn main() -> i32 {
    for c in "hello world!!!!!".chars() {
        println!("{}", c);
        yield_now();
    }
    0
}
