mod io;
mod mm;
mod process;

extern crate alloc;

use crate::syscall::{
    io::sys_write,
    mm::mmap,
    process::{sys_exit, sys_task_info, sys_yield},
};
use crate::task::prelude::{TaskInfo, get_current_task_id};
use crate::{log, warn};

const SYSCALL_WRITE: usize = 64;
const SYSCALL_MMAP: usize = 90;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_TASK_INFO: usize = (1 << 63) | 1;

pub fn syscall_handler(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_MMAP => mmap(args[0], args[1], args[2]),
        SYSCALL_MUNMAP => mm::munmap(args[0], args[1]),
        SYSCALL_EXIT => sys_exit(args[0] as isize),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_TASK_INFO => sys_task_info(args[0], args[1] as *mut TaskInfo),
        _ => panic!("Unknown syscall, id={syscall_id}, args={args:?}."),
    }
}

fn log_failed_copy_from(src: *const u8, len: usize, failed_len: usize) {
    warn!(
        "Task {:?}: Failed to copy from user, src={:#x}, len={}, failed_len={}",
        get_current_task_id(),
        src.addr(),
        len,
        failed_len,
    );
}

fn log_failed_copy_to(dst: *mut u8, len: usize, failed_len: usize) {
    warn!(
        "Task {:?}: Failed to copy to user, dst={:#x}, len={}, failed_len={}",
        get_current_task_id(),
        dst.addr(),
        len,
        failed_len,
    );
}
