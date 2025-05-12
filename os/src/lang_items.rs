use core::arch::asm;

use crate::{println, sbi::shutdown};

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("Panicked: {}", info.message());
    }
    unsafe { print_stack_trace() };
    shutdown(true)
}

/// Prints a stack trace of the current execution.
///
/// # Safety
/// This function relies on the saved frame pointers to work correctly.
unsafe fn print_stack_trace() {
    println!("#------- Stack Trace (most recent first) -------#");
    let mut fp: *const usize;

    unsafe { asm!("mv {}, fp", lateout(reg) fp) };
    while !fp.is_null() {
        let ra = unsafe { fp.offset(-1).read() };
        fp = unsafe { fp.offset(-2).read() as *const usize };
        println!("| fp {:#018x}, ra {:#018x}  |", fp as usize, ra);
    }
    println!("#-----------------------------------------------#");
}
