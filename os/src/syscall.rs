mod io;

extern crate alloc;

use crate::mm::prelude::{check_u_va_range, copy_to_user};
use crate::syscall::io::sys_write;
use crate::task::prelude::{
    TaskInfo, TaskState, exchange_current_task_state, get_current_task_id, get_task_info,
    run_next_task,
};
use crate::{info, log, warn};

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;

const SYSCALL_TASK_INFO: usize = (1 << 63) | 1;

pub fn syscall_handler(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as isize),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_TASK_INFO => sys_task_info(args[0], args[1] as *mut TaskInfo),
        _ => panic!("Unknown syscall, id={syscall_id}, args={args:?}"),
    }
}

fn sys_exit(exit_code: isize) -> isize {
    let task_id = get_current_task_id().expect("sys_exit when no task is running");

    let state = exchange_current_task_state(TaskState::Running, TaskState::Exited);
    if let Err(state) = state {
        panic!("Task {:?}: Expected running but got {:?}", task_id, state)
    }

    info!("Task {:?}: Exited with code {}", task_id, exit_code);
    run_next_task();
    exit_code
}

fn sys_yield() -> isize {
    let task_id = get_current_task_id().expect("sys_yield when no task is running");

    let state = exchange_current_task_state(TaskState::Running, TaskState::Ready);
    if let Err(state) = state {
        panic!("Task {:?}: Expected running but got {:?}", task_id, state)
    }

    info!("Task {:?}: Yield", task_id);
    run_next_task();
    0
}

fn sys_task_info(task_id: usize, data: *mut TaskInfo) -> isize {
    let dst = data as *mut u8;
    let len = size_of::<TaskInfo>();

    if !check_u_va_range(dst.addr(), len) {
        log_failed_copy_to(dst, len, len);
        return -1;
    }

    if let Some(task_info) = get_task_info(task_id) {
        let src = (&raw const task_info) as *const u8;
        let failed_len = unsafe { copy_to_user(src, dst, len) };

        if failed_len == 0 {
            0
        } else {
            log_failed_copy_to(dst, len, failed_len);
            -1
        }
    } else {
        -1
    }
}

fn log_failed_copy_from(src: *const u8, len: usize, failed_len: usize) {
    warn!(
        "Task {:?}: Failed to copy from user, src={:#x}, len={}, failed_len={}",
        get_current_task_id().expect("Expect a running task"),
        src.addr(),
        len,
        failed_len,
    );
}

fn log_failed_copy_to(dst: *mut u8, len: usize, failed_len: usize) {
    warn!(
        "Task {:?}: Failed to copy to user, dst={:#x}, len={}, failed_len={}",
        get_current_task_id().expect("Expect a running task"),
        dst.addr(),
        len,
        failed_len,
    );
}
