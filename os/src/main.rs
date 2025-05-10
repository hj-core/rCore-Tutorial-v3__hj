#![no_std]
#![no_main]

mod batch;
mod console;
mod lang_items;
mod sbi;

use core::arch::{asm, global_asm};

use batch::AppManager;
use console::log;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_apps.S"));

#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    clear_bss();
    print!("{} {} cleared\n", "[   OS] ", "bss");

    log::init();
    log_kernel_layout();
    log_apps_layout();

    test_riscv_macros();

    AppManager::install_app(0);

    println!("{} Hello world, {}!", "[   OS] ", "everybody");
    panic!("Shutdown machine!");
}

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

fn clear_bss() {
    (bss_start as usize..bss_end as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) };
    });
}

fn log_kernel_layout() {
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

fn log_apps_layout() {
    let total_apps = AppManager::get_total_apps();
    for i in 0..total_apps {
        let app_start = AppManager::get_app_data_start(i);
        let app_end = AppManager::get_app_data_end(i);
        let size = app_end - app_start;
        debug!("app_{} [{:#x}, {:#x}) size={}", i, app_start, app_end, size);
    }
}

fn test_riscv_macros() {
    test_riscv_csrr();
    test_riscv_csrw();
    test_riscv_csrrc();
    test_riscv_csrrs();
}

fn test_riscv_csrr() {
    const STVAL_NO: usize = 0x143;
    let mut stval_val: usize;

    unsafe { asm!("csrw {csr}, {rd}", csr = const STVAL_NO, rd = in(reg) 20) };
    riscv::csrr!(STVAL_NO, stval_val);
    assert_eq!(20, stval_val);

    unsafe { asm!("csrw {csr}, {rd}", csr = const STVAL_NO,rd = in(reg) 55) };
    riscv::csrr!(STVAL_NO, stval_val);
    assert_eq!(55, stval_val);

    debug!("riscv::csrr worked correctly");
}

fn test_riscv_csrw() {
    const STVAL_NO: usize = 0x143;
    let mut stval_val: usize;

    riscv::csrw!(STVAL_NO, 256);
    unsafe { asm!("csrr {rd}, {csr}", rd = lateout(reg) stval_val, csr = const STVAL_NO) };
    assert_eq!(256, stval_val);

    riscv::csrw!(STVAL_NO, 996);
    unsafe { asm!("csrr {rd}, {csr}", rd = lateout(reg) stval_val, csr = const STVAL_NO) };
    assert_eq!(996, stval_val);

    debug!("riscv::csrw worked correctly");
}

fn test_riscv_csrrc() {
    const STVAL_NO: usize = 0x143;
    let mut old_stval_val: usize;
    let mut new_stval_val: usize;

    // Clear a single set bit
    unsafe { asm!("csrw {csr}, {rd}", csr = const STVAL_NO, rd = in(reg) 0b1111) };
    riscv::csrrc!(STVAL_NO, old_stval_val, 0b1);
    assert_eq!(0b1111, old_stval_val);
    unsafe { asm!("csrr {rd}, {csr}", rd = lateout(reg) new_stval_val, csr = const STVAL_NO) };
    assert_eq!(0b1110, new_stval_val);

    // Clear multiple bits including set bits and an unset bit
    unsafe { asm!("csrw {csr}, {rd}", csr = const STVAL_NO, rd = in(reg) 0b01111) };
    riscv::csrrc!(STVAL_NO, old_stval_val, 0b11010);
    assert_eq!(0b01111, old_stval_val);
    unsafe { asm!("csrr {rd}, {csr}", rd = lateout(reg) new_stval_val, csr = const STVAL_NO) };
    assert_eq!(0b0101, new_stval_val);

    debug!("riscv::csrrc worked correctly");
}

fn test_riscv_csrrs() {
    const STVAL_NO: usize = 0x143;
    let mut old_stval_val: usize;
    let mut new_stval_val: usize;

    // Set a single unset bit
    unsafe { asm!("csrw {csr}, {rd}", csr = const STVAL_NO, rd= in(reg) 0b1001) };
    riscv::csrrs!(STVAL_NO, old_stval_val, 0b10);
    assert_eq!(0b1001, old_stval_val);
    unsafe { asm!("csrr {rd}, {csr}", rd = lateout(reg) new_stval_val, csr = const STVAL_NO) };
    assert_eq!(0b1011, new_stval_val);

    // Set mutliple unset bits and a set bit
    unsafe { asm!("csrw {csr}, {rd}", csr = const STVAL_NO, rd= in(reg) 0b0100) };
    riscv::csrrs!(STVAL_NO, old_stval_val, 0b1111);
    assert_eq!(0b0100, old_stval_val);
    unsafe { asm!("csrr {rd}, {csr}", rd = lateout(reg) new_stval_val, csr = const STVAL_NO) };
    assert_eq!(0b1111, new_stval_val);

    debug!("riscv::csrrs worked correctly");
}
