#![no_std]
#![no_main]

mod console;
mod lang_items;
mod sbi;

use core::arch::global_asm;

use console::log;

global_asm!(include_str!("entry.asm"));

#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    clear_bss();
    let pronouns = "Everybody";
    print!("Hello World, {pronouns}!");
    println!();
    print!("Rpeat, ");
    println!("Hello World, {}, {}!!", pronouns, pronouns);
    log!(log::Level::NONE, "This is a {} message", "NONE");
    log!(log::Level::ERROR, "This is a {} message", "ERROR");
    log!(log::Level::WARN, "This is a {} message", "WARN");
    log!(log::Level::INFO, "This is a {} message", "INFO");
    log!(log::Level::DEBUG, "This is a {} message", "DEBUG");
    log!(log::Level::TRACE, "This is a {} message", "TRACE");
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
