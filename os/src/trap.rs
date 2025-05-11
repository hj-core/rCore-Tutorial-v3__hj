use core::arch::global_asm;
use riscv::regs::{scause, stval, stvec};

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
    let scause_val = scause::read();
    let cause = scause::match_cause(scause_val);
    let stval_val = stval::read();

    if matches!(cause, scause::Cause::Unknown) {
        panic!("Unknown trap, scause={scause_val:x}, stval={stval_val:x}")
    } else {
        panic!("Trap: {cause:?}, stval={stval_val:x}");
    }
}
