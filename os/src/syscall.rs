use core::{slice, str};

use crate::{batch::AppManager, print, println};

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
        println!(
            "[KERNEL] User attempts to write to unsupported file descriptor: {}",
            fd
        );
        return -1;
    }

    if !AppManager::can_app_read_addr(buf.addr())
        || !AppManager::can_app_read_addr(buf.addr() + count - 1)
    {
        println!("[KERNEL] User attempts to read a memory address without permission");
        return -1;
    }

    let buf = unsafe { slice::from_raw_parts(buf, count) };
    let str = str::from_utf8(buf).unwrap();
    print!("{str}");
    count as isize
}

fn sys_exit(exit_code: isize) -> ! {
    println!("[KERNEL] Application exited with code {}", exit_code);
    AppManager::run_next_app()
}

fn sys_task_info() -> isize {
    let app_index = AppManager::get_curr_app_index();
    let app_name = AppManager::get_app_name(app_index);

    println!(
        "[KERNEL] Running Task {{ index: {}, name: {} }}",
        app_index, app_name
    );
    0
}
