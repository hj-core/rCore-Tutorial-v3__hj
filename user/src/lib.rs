#![no_std]

pub mod console;
mod lang_items;
mod syscall;

use syscall::{sys_exit, sys_task_info, sys_write};

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

pub fn get_task_info() -> isize {
    sys_task_info()
}
