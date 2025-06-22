mod heap_alloc;
mod page_alloc;
pub mod prelude;
mod sv39;
mod vm;

use crate::{debug, error, info, log, trace, warn};

/// The (base, size) pairs of the QEMU virt machine MMIO scheme.
/// For more details, see [here].
///
/// [here]: https://github.com/qemu/qemu/blob/master/hw/riscv/virt.c#L82
const QEMU_VIRT_MMIO: &[(usize, usize)] = &[
    (0x0010_0000, 0x0000_1000), // VIRT_TEST
    (0x0010_1000, 0x0000_1000), // VIRT_RTC
    (0x0200_0000, 0x0001_0000), // VIRT_CLINT
    (0x0c00_0000, 0x0060_0000), // VIRT_PLIC
    (0x1000_0000, 0x0000_0100), // VIRT_UART0
    (0x1001_0000, 0x0000_1000), // VIRT_VIRTIO
];

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
    heap_alloc::init();
    vm::print_kernel_space();
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
