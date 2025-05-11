use core::arch::global_asm;
use riscv::regs::{scause, stval, stvec};

use crate::{print, println};

global_asm!(include_str!("trap/trap.S"));

pub fn init() {
    unsafe extern "C" {
        unsafe fn __stvec();
    }
    let stvec_ok = stvec::install(__stvec as usize, stvec::Mode::Direct);
    assert!(stvec_ok, "Failed to install stvec");
}

struct TrapContext {
    /// Stores the values of registers x0 through x31.
    ///
    /// Please note that the actual implementation may skip storing some
    /// register values; thus the values at those indices are invalid.
    x: [usize; 32],
    sstatus: usize,
    sepc: usize,
}

#[unsafe(no_mangle)]
fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause_val = scause::read();
    let cause = scause::match_cause(scause_val);
    let stval_val = stval::read();

    if matches!(cause, scause::Cause::UserEnvironmentCall) {
        let x10 = cx.x[10];
        let x11 = cx.x[11];
        let x12 = cx.x[12];
        let x17 = cx.x[17];
        println!(
            "UserEnvironmentCall:\nx10={}, x11={:#x}, x12={}, x17={}, sstatus={:#x}, sepc={:#x}",
            x10, x11, x12, x17, cx.sstatus, cx.sepc,
        );

        const SYSCALL_WRITE: usize = 64;
        if x17 == SYSCALL_WRITE && x10 == 1 {
            let slice = unsafe { core::slice::from_raw_parts(x11 as *const u8, x12) };
            let str = core::str::from_utf8(slice).unwrap();
            print!("User app attempts to print: {str}");
        }
    }

    if matches!(cause, scause::Cause::Unknown) {
        panic!("Unknown trap, scause={scause_val:x}, stval={stval_val:x}")
    } else {
        panic!("Trap: {cause:?}, stval={stval_val:x}");
    }
}
