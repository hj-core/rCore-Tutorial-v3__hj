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

    println!("Get task info of task {} into a null pointer", 1);
    let null_ptr = 0 as *mut TaskInfo;
    let result = get_task_info(1, null_ptr);
    assert_eq!(result, -1);
    println!("Kernel survived writing to null pointer, good");

    0
}
