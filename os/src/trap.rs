use core::arch::global_asm;
use riscv::regs::stvec;

global_asm!(include_str!("trap/trap.S"));

pub fn init() {
    unsafe extern "C" {
        unsafe fn __stvec();
    }
    let stvec_ok = stvec::install(__stvec as usize, stvec::Mode::Direct);
    assert!(stvec_ok, "Failed to install stvec");
}

#[unsafe(no_mangle)]
fn trap_handler() {
    panic!("os caught a trap!")
}
