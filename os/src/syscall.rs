use core::{slice, str};

use crate::{
    info, log, print,
    task::prelude::{
        TaskInfo, TaskState, exchange_recent_task_state, get_recent_task_index, get_task_info,
        get_task_name, run_next_task,
    },
    warn,
};

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;

const SYSCALL_TASK_INFO: usize = (1 << 63) | 1;

const FD_STDOUT: usize = 1;

pub fn syscall_handler(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as isize),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_TASK_INFO => sys_task_info(args[0], args[1] as *mut TaskInfo),
        _ => panic!("Unknown syscall, id={syscall_id}, args={args:?}"),
    }
}

fn sys_write(fd: usize, buf: *const u8, count: usize) -> isize {
    let task_index = get_recent_task_index();
    let task_name = get_task_name(task_index);

    if fd != FD_STDOUT {
        warn!(
            "Task {{ index: {}, name: {} }} attempted to write to unsupported file descriptor {}",
            task_index, task_name, fd
        );
        return -1;
    }

    let buf = unsafe { slice::from_raw_parts(buf, count) };
    let str = str::from_utf8(buf).unwrap();
    print!("{str}");
    count as isize
}

fn sys_exit(exit_code: isize) -> isize {
    let task_index = get_recent_task_index();
    let task_name = get_task_name(task_index);

    let state = exchange_recent_task_state(TaskState::Running, TaskState::Exited);
    if let Err(state) = state {
        panic!(
            "Task {{ index: {}, name: {} }} expected Running but got {:?}",
            task_index, task_name, state
        )
    }

    info!(
        "Task {{ index: {}, name: {} }} exited with code {}",
        task_index, task_name, exit_code
    );
    run_next_task();
    exit_code
}

fn sys_yield() -> isize {
    let task_index = get_recent_task_index();
    let task_name = get_task_name(task_index);

    let state = exchange_recent_task_state(TaskState::Running, TaskState::Ready);
    if let Err(state) = state {
        panic!(
            "Task {{ index: {}, name: {} }} expected Running but got {:?}",
            task_index, task_name, state
        )
    }

    run_next_task();
    0
}

fn sys_task_info(task_index: usize, data: *mut TaskInfo) -> isize {
    let task_info = get_task_info(task_index);
    unsafe { data.write(task_info) }
    0
}
