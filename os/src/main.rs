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
use task::prelude::{get_app_data_end, get_app_data_start, get_app_name, get_total_apps};

global_asm!(include_str!("entry.S"));
global_asm!(include_str!("link_apps.S"));

#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    mm_p::clear_bss();

    log::init();
    log_apps_layout();
    mm_p::log_kernel_layout();

    trap::init();

    task::start();
}

fn log_apps_layout() {
    let total_apps = get_total_apps();

    for i in 0..total_apps {
        let app_start = get_app_data_start(i);
        let app_end = get_app_data_end(i);
        let app_size = app_end - app_start;
        let app_name = get_app_name(i);

        debug!(
            "app_{} [{:#x}, {:#x}) size={}, name={}",
            i, app_start, app_end, app_size, app_name
        );
    }
}
