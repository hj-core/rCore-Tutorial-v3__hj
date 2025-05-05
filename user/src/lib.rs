#![no_std]

use syscall::sys_write;

mod lang_items;
mod syscall;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    clear_bas();
    write(1, "Hello World!\n".as_bytes());
    unsafe { main() };
    panic!()
}

fn clear_bas() {
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
