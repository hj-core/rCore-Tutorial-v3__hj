use crate::mm::prelude::{check_u_va_range, copy_to_user};
use crate::task::prelude::{
    TaskInfo, TaskState, exchange_current_task_state, get_current_task_id, get_task_info,
    run_next_task,
};
use crate::{info, log};

use crate::syscall::log_failed_copy_to;

pub(super) fn sys_exit(exit_code: isize) -> isize {
    let task_id = get_current_task_id().expect("sys_exit when no task is running");

    let state = exchange_current_task_state(TaskState::Running, TaskState::Exited);
    if let Err(state) = state {
        panic!("Task {:?}: Expected running but got {:?}", task_id, state)
    }

    info!("Task {:?}: Exited with code {}", task_id, exit_code);
    run_next_task();
    exit_code
}

pub(super) fn sys_yield() -> isize {
    let task_id = get_current_task_id().expect("sys_yield when no task is running");

    let state = exchange_current_task_state(TaskState::Running, TaskState::Ready);
    if let Err(state) = state {
        panic!("Task {:?}: Expected running but got {:?}", task_id, state)
    }

    info!("Task {:?}: Yield", task_id);
    run_next_task();
    0
}

pub(super) fn sys_task_info(task_id: usize, data: *mut TaskInfo) -> isize {
    let dst = data as *mut u8;
    let len = size_of::<TaskInfo>();

    if !check_u_va_range(dst.addr(), len) {
        log_failed_copy_to(dst, len, len);
        return -1;
    }

    let task_info = get_task_info(task_id);
    if task_info.is_none() {
        return -1;
    }

    let task_info = task_info.unwrap();
    let src = (&raw const task_info) as *const u8;
    let failed_len = unsafe { copy_to_user(src, dst, len) };

    if failed_len == 0 {
        0
    } else {
        log_failed_copy_to(dst, len, failed_len);
        -1
    }
}
