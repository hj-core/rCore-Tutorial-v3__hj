use core::{slice, str};

use crate::{batch::AppManager, print, println};

const SYSCALL_WRITE: usize = 64;
const FD_STD: usize = 1;

const SYSCALL_EXIT: usize = 93;

pub fn syscall_handler(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as isize),
        _ => panic!("Unknown syscall, id={syscall_id}, args={args:?}"),
    }
}

fn sys_write(fd: usize, buf: *const u8, count: usize) -> isize {
    if fd != FD_STD {
        panic!("Unsupported file descriptor: {fd}");
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
