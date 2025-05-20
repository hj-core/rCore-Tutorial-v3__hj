use core::{slice, str};

use crate::{
    info, log, print, println,
    task::{AppRunner, loader},
    warn,
};

const SYSCALL_WRITE: usize = 64;
const FD_STDOUT: usize = 1;

const SYSCALL_EXIT: usize = 93;
const SYSCALL_TASK_INFO: usize = (1 << 63) | 1;

pub fn syscall_handler(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as isize),
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

    let app_index = AppRunner::get_curr_app_index();
    if !loader::can_app_read_addr(app_index, buf.addr())
        || !loader::can_app_read_addr(app_index, buf.addr() + count - 1)
    {
        warn!("User attempts to read a memory address without permission");
        return -1;
    }

    let buf = unsafe { slice::from_raw_parts(buf, count) };
    let str = str::from_utf8(buf).unwrap();
    print!("{str}");
    count as isize
}

fn sys_exit(exit_code: isize) -> ! {
    info!("Application exited with code {}", exit_code);
    AppRunner::run_next_app()
}

fn sys_task_info() -> isize {
    let app_index = AppRunner::get_curr_app_index();
    let app_name = loader::get_app_name(app_index);

    println!(
        "Running Task {{ index: {}, name: {} }}",
        app_index, app_name
    );
    0
}
