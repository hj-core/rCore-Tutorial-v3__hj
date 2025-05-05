#![no_std]

mod lang_items;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    clear_bas();
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
