use core::{slice, str};

use crate::{
    info, log, print, println,
    task::prelude::{
        TaskState, can_app_read_addr, change_current_task_state, get_app_name,
        get_current_app_index, is_current_task_running, run_next_app,
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
        SYSCALL_TASK_INFO => sys_task_info(),
        _ => panic!("Unknown syscall, id={syscall_id}, args={args:?}"),
    }
}

fn sys_write(fd: usize, buf: *const u8, count: usize) -> isize {
    if fd != FD_STDOUT {
        warn!(
            "User attempts to write to unsupported file descriptor: {}",
            fd
        );
        return -1;
    }

    let app_index = get_current_app_index();
    if !can_app_read_addr(app_index, buf.addr())
        || !can_app_read_addr(app_index, buf.addr() + count - 1)
    {
        warn!("User attempts to read a memory address without permission");
        return -1;
    }

    let buf = unsafe { slice::from_raw_parts(buf, count) };
    let str = str::from_utf8(buf).unwrap();
    print!("{str}");
    count as isize
}

fn sys_exit(exit_code: isize) -> isize {
    assert!(is_current_task_running());
    change_current_task_state(TaskState::Exited);

    info!("Application exited with code {}", exit_code);
    run_next_app();
    exit_code
}

fn sys_yield() -> isize {
    assert!(is_current_task_running());
    change_current_task_state(TaskState::Ready);
    run_next_app();
    0
}

fn sys_task_info() -> isize {
    let app_index = get_current_app_index();
    let app_name = get_app_name(app_index);

    println!(
        "Running Task {{ index: {}, name: {} }}",
        app_index, app_name
    );
    0
}
