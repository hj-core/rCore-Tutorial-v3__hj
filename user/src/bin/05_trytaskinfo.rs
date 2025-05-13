#![no_std]
#![no_main]

extern crate user_lib;

use user_lib::{get_task_info, println};

#[unsafe(no_mangle)]
fn main() -> i32 {
    println!("Try to get the information of current running app");
    get_task_info();
    println!("Kernel should have display the information of current running app");
    0
}
