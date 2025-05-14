use core::arch::asm;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_TASK_INFO: usize = (1 << 63) | 1;

fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut result: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => result,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        );
    }
    result
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(xstate: i32) -> isize {
    syscall(SYSCALL_EXIT, [xstate as usize, 0, 0])
}

pub fn sys_task_info() -> isize {
    syscall(SYSCALL_TASK_INFO, [0, 0, 0])
}
