#![no_std]
#![no_main]

mod lang_items;
mod sbi;

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));

#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    clear_bss();
    sbi::console_putchar('H' as usize);
    loop {}
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
