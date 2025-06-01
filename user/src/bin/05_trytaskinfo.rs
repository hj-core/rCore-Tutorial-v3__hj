#![no_std]
#![no_main]

extern crate user_lib;

use user_lib::{
    get_task_info, println,
    task::{TaskInfo, TaskState},
};

const SYSCALL_WRITE: usize = 64;
const SYSCALL_YIELD: usize = 124;

const SYSCALL_TASK_INFO: usize = (1 << 63) | 1;

#[unsafe(no_mangle)]
fn main() -> i32 {
    let mut task_info = TaskInfo::new_placeholder();

    println!("Get task info of task 0");
    get_task_info(0, &raw mut task_info);
    assert!(task_info.state == TaskState::Ready);
    assert!(
        task_info
            .stastics
            .syscall_counts
            .iter()
            .find(|(id, _times)| *id == SYSCALL_YIELD)
            .is_some_and(|(_, times)| *times == 1)
    );
    assert!(
        task_info
            .stastics
            .syscall_counts
            .iter()
            .find(|(id, _times)| *id == SYSCALL_WRITE)
            .is_some_and(|(_, times)| *times > 1)
    );

    println!("Get task info of task 1");
    get_task_info(1, &raw mut task_info);
    assert!(task_info.state == TaskState::Killed);

    println!("Get task info of task 3");
    get_task_info(3, &raw mut task_info);
    assert!(task_info.state == TaskState::Killed);
    assert!(task_info.stastics.mtime_first_run_start > 0);
    assert_eq!(
        task_info.stastics.mtime_first_run_start,
        task_info.stastics.mtime_last_run_start
    );

    println!("Get task info of task 4");
    get_task_info(4, &raw mut task_info);
    assert!(task_info.state == TaskState::Killed);
    assert_eq!(task_info.stastics.mtime_total_waiting, 0);
    assert_eq!(task_info.stastics.switch_count, 1);

    println!("Get task info of self");
    get_task_info(5, &raw mut task_info);
    assert!(task_info.state == TaskState::Running);
    assert!(
        task_info
            .stastics
            .syscall_counts
            .iter()
            .find(|(id, _times)| *id == SYSCALL_TASK_INFO)
            .is_some_and(|(_, times)| *times == 5)
    );

    0
}
