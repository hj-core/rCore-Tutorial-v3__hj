use core::arch::global_asm;
use riscv::regs::{
    scause::{self, Cause},
    sstatus, stval, stvec,
};

use crate::{log, syscall, task::runner, warn};

global_asm!(include_str!("trap.S"));

#[derive(Debug)]
#[repr(C)]
pub struct TrapContext {
    /// Stores the values of registers x0 through x31.
    ///
    /// Please note that the actual implementation may skip storing some
    /// register values; thus the values at those indices are invalid.
    x: [usize; 32],
    #[allow(dead_code)]
    sstatus: usize,
    sepc: usize,
}

impl TrapContext {
    pub fn new_app_context(app_entry_addr: usize, user_sp: usize) -> Self {
        let sstatus = sstatus::set_spp_user();
        let mut result = Self {
            x: [0; 32],
            sstatus,
            sepc: app_entry_addr,
        };
        result.x[2] = user_sp;
        result
    }
}

pub fn init() {
    unsafe extern "C" {
        unsafe fn __stvec();
    }
    let stvec_ok = stvec::install(__stvec as usize, stvec::Mode::Direct);
    assert!(stvec_ok, "Failed to install stvec");
}

#[unsafe(no_mangle)]
fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause_val = scause::read();
    let cause = scause::match_cause(scause_val);
    let stval_val = stval::read();

    match cause {
        Cause::UserEnvironmentCall => {
            cx.sepc += 4;
            cx.x[10] = syscall::syscall_handler(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Cause::StoreOrAmoAccessFault | Cause::StoreOrAmoPageFault => {
            warn!("PageFault in application, kernel killed it.");
            runner::run_next_app();
        }
        Cause::IllegalInstruction => {
            warn!("IllegalInstruction in application, kernel killed it.");
            runner::run_next_app();
        }
        Cause::Unknown => {
            panic!("Unknown trap, scause={scause_val:x}, stval={stval_val:x}, context={cx:?}")
        }
        _ => {
            panic!("Trap: {cause:?}, stval={stval_val:x}, context={cx:#x?}");
        }
    }
    cx
}
