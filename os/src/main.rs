#![no_std]
#![no_main]

mod console;
mod lang_items;
mod sbi;

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));

#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    clear_bss();
    let pronouns = "Everybody";
    print!("Hello World, {pronouns}!");
    println!();
    print!("Rpeat, ");
    println!("Hello World, {}, {}!!", pronouns, pronouns);
    panic!("Shutdown machine!");
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
