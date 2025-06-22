mod heap_alloc;
mod page_alloc;
pub mod prelude;
mod sv39;
mod vm;

use crate::{debug, error, info, log, trace, warn};

unsafe extern "C" {
    unsafe fn kernel_start();
    unsafe fn text_start();
    unsafe fn text_end();
    unsafe fn rodata_start();
    unsafe fn rodata_end();
    unsafe fn data_start();
    unsafe fn data_end();
    unsafe fn boot_stack_lower_bound();
    unsafe fn boot_stack_top();
    unsafe fn bss_start();
    unsafe fn bss_end();
    unsafe fn kernel_end();
}

pub(crate) fn get_kernel_end() -> usize {
    kernel_end as usize
}

pub(crate) fn init() {
    clear_bss();
    log_kernel_layout();
    heap_alloc::init();
}

fn clear_bss() {
    (bss_start as usize..bss_end as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) };
    });
}

pub(crate) fn log_kernel_layout() {
    log!(
        log::Level::NONE,
        " kernel     [{:#x}, {:#x}) size={}",
        kernel_start as usize,
        kernel_end as usize,
        kernel_end as usize - kernel_start as usize
    );
    trace!(
        ".text       [{:#x}, {:#x}) size={}",
        text_start as usize,
        text_end as usize,
        text_end as usize - text_start as usize
    );
    debug!(
        ".rodata     [{:#x}, {:#x}) size={}",
        rodata_start as usize,
        rodata_end as usize,
        rodata_end as usize - rodata_start as usize
    );
    info!(
        ".data       [{:#x}, {:#x}) size={}",
        data_start as usize,
        data_end as usize,
        data_end as usize - data_start as usize
    );
    warn!(
        ".boot_stack [{:#x}, {:#x}) size={}",
        boot_stack_lower_bound as usize,
        boot_stack_top as usize,
        boot_stack_top as usize - boot_stack_lower_bound as usize
    );
    error!(
        ".bss        [{:#x}, {:#x}) size={}",
        bss_start as usize,
        bss_end as usize,
        bss_end as usize - bss_start as usize
    );
}
