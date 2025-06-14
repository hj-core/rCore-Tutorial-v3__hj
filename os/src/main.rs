#![no_std]
#![no_main]

mod console;
mod lang_items;
mod mm;
mod sbi;
mod sync;
mod syscall;
mod task;
mod timer;
mod trap;

use core::arch::global_asm;

use console::log;
use mm::prelude as mm_p;
use task::prelude as task_p;

global_asm!(include_str!("entry.S"));
global_asm!(include_str!("link_apps.S"));

#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    mm_p::clear_bss();

    log::init();
    mm_p::log_kernel_layout();
    task_p::log_apps_layout();

    trap::init();

    task::start();
}
