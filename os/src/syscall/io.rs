extern crate alloc;

use alloc::vec;
use core::str;

use crate::mm::prelude::{check_u_va_range, copy_from_user};
use crate::syscall::log_failed_copy_from;
use crate::task::prelude::get_current_task_id;
use crate::{log, print, warn};

const FD_STDOUT: usize = 1;

pub(super) fn sys_write(fd: usize, buf: *const u8, count: usize) -> isize {
    if fd != FD_STDOUT {
        warn!(
            "Task {:?}: Unsupported file descriptor {}",
            get_current_task_id().expect("Expect a running task"),
            fd
        );
        return -1;
    }

    if !check_u_va_range(buf.addr(), count) {
        log_failed_copy_from(buf, count, count);
        return -1;
    }

    let mut dst = vec![0; count];
    let failed_len = unsafe { copy_from_user(buf, dst.as_mut_ptr(), count) };

    if failed_len != 0 {
        log_failed_copy_from(buf, count, failed_len);
        return -1;
    }

    let str = str::from_utf8(&dst).unwrap();
    print!("{str}");

    count as isize
}
