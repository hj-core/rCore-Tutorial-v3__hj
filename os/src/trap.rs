use core::arch::global_asm;
use riscv::regs::{
    scause::{self, Cause},
    sstatus, stval, stvec,
};

use crate::{
    log, syscall,
    task::prelude::{TaskState, change_current_task_state, is_current_task_running, run_next_app},
    warn,
};

global_asm!(include_str!("trap.S"));

unsafe extern "C" {
    pub(super) unsafe fn __restore(context: *mut TrapContext);
}

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
fn trap_handler(context: &mut TrapContext) -> &mut TrapContext {
    let scause_val = scause::read();
    let cause = scause::match_cause(scause_val);
    let stval_val = stval::read();

    match cause {
        Cause::UserEnvironmentCall => {
            context.sepc += 4;
            context.x[10] = syscall::syscall_handler(
                context.x[17],
                [context.x[10], context.x[11], context.x[12]],
            ) as usize;
        }

        Cause::StoreOrAmoAccessFault | Cause::StoreOrAmoPageFault => {
            assert!(is_current_task_running());
            change_current_task_state(TaskState::Killed);
            warn!("PageFault in application, kernel killed it.");
            run_next_app();
        }

        Cause::IllegalInstruction => {
            assert!(is_current_task_running());
            change_current_task_state(TaskState::Killed);
            warn!("IllegalInstruction in application, kernel killed it.");
            run_next_app();
        }

        Cause::Unknown => {
            panic!(
                "Unknown trap, scause={scause_val:#x}, stval={stval_val:#x}, context={context:#?}"
            )
        }

        _ => {
            panic!("Trap: {cause:?}, stval={stval_val:#x}, context={context:#x?}");
        }
    }
    context
}
