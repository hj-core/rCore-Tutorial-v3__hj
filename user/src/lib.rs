#![no_std]

pub mod console;
mod lang_items;
mod syscall;
pub mod task;

use syscall::{sys_exit, sys_task_info, sys_write, sys_yield};
use task::TaskInfo;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    clear_bss();
    unsafe { sys_exit(main()) };
    unreachable!()
}

fn clear_bss() {
    unsafe extern "C" {
        unsafe fn bss_start();
        unsafe fn bss_end();
    }
    (bss_start as usize..bss_end as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) };
    });
}

unsafe extern "Rust" {
    unsafe fn main() -> i32;
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}

pub fn yield_now() -> isize {
    sys_yield()
}

pub fn get_task_info(task_id: usize, data: *mut TaskInfo) -> isize {
    sys_task_info(task_id, data)
}
