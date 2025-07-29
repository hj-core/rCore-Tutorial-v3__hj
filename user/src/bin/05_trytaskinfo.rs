#![no_std]
#![no_main]

extern crate user_lib;

use user_lib::task::TaskInfo;
use user_lib::{get_task_info, println};

#[unsafe(no_mangle)]
fn main() -> i32 {
    let mut task_info = TaskInfo::new_placeholder();

    for task_id in 1..=6 {
        println!("Get task info of task {}", task_id);
        get_task_info(1, &raw mut task_info);
        println!("{:?}", task_info);
    }

    0
}
