use core::{slice, str};

use riscv::regs::sstatus;

use crate::task::prelude::{
    TaskInfo, TaskState, exchange_current_task_state, get_current_task_id, get_task_info,
    run_next_task,
};
use crate::{info, log, print, warn};

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
    if fd != FD_STDOUT {
        let task_id = get_current_task_id();
        warn!(
            "Task {:?} attempted write to an unsupported file descriptor {}",
            task_id, fd
        );
        return -1;
    }

    sstatus::set_sum_permit();
    let buf = unsafe { slice::from_raw_parts(buf, count) };
    let str = str::from_utf8(buf).unwrap();
    print!("{str}");
    sstatus::set_sum_deny();
    count as isize
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
    if let Some(task_info) = get_task_info(task_id) {
        sstatus::set_sum_permit();
        unsafe { data.write(task_info) }
        sstatus::set_sum_deny();
        0
    } else {
        -1
    }
}
