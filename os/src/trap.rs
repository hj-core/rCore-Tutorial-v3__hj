use core::arch::global_asm;
use riscv::regs::{
    scause::{self, Cause},
    sie, sstatus, stval, stvec,
};

use crate::{
    info, log, syscall,
    task::{
        get_task_name,
        prelude::{TaskState, exchange_recent_task_state, get_recent_task_index, run_next_task},
    },
    warn,
};

global_asm!(include_str!("trap.S"));

unsafe extern "C" {
    // Defined in trap.S
    pub(super) unsafe fn __restore();
}

#[derive(Debug)]
#[repr(C)]
pub struct TrapContext {
    /// Stores the values of registers x0 through x31.
    ///
    /// Please note that the actual implementation may skip storing some
    /// register values; thus the values at those indices are invalid.
    x: [usize; 32],
    sstatus: usize,
    sepc: usize,
}

impl TrapContext {
    pub fn new_init_context(entry_addr: usize, user_sp: usize) -> Self {
        let sstatus = sstatus::set_spp_user();

        let mut result = Self {
            x: [0; 32],
            sstatus,
            sepc: entry_addr,
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

    enable_interrupts();
    enable_timer_interrupts();
}

/// `enable_interrupts` enables all interrupts in supervisor mode. This provides
/// overall control over interrupt behavior.
fn enable_interrupts() {
    sstatus::set_sie();
}

/// `enable_timer_interrupts` enables the timer interrupts in supervisor mode.
/// This provides fine control over interrupt behavior.
fn enable_timer_interrupts() {
    sie::set_stie();
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
            exchange_recent_task_state(TaskState::Running, TaskState::Killed)
                .expect("Expected the current TaskState to be Running");

            let task_index = get_recent_task_index();
            let taks_name = get_task_name(task_index);
            warn!(
                "Task {{ index: {}, name: {} }} PageFault , kernel killed it.",
                task_index, taks_name
            );
            run_next_task();
        }

        Cause::IllegalInstruction => {
            exchange_recent_task_state(TaskState::Running, TaskState::Killed)
                .expect("Expected the current TaskState to be Running");

            let task_index = get_recent_task_index();
            let taks_name = get_task_name(task_index);
            warn!(
                "Task {{ index: {}, name: {} }} IllegalInstruction, kernel killed it.",
                task_index, taks_name
            );
            run_next_task();
        }

        Cause::SupervisorTimerInterrupt => {
            info!("Caught a SupervisorTimerInterrupt");

            exchange_recent_task_state(TaskState::Running, TaskState::Ready)
                .expect("Expected the current TaskState to be Running");
            run_next_task();
        }

        Cause::Unknown => {
            let task_index = get_recent_task_index();
            let task_name = get_task_name(task_index);
            panic!(
                "Task {{ index: {task_index}, name: {task_name} }} unknown trap, scause={scause_val:#x}, stval={stval_val:#x}, context={context:#?}"
            )
        }

        _ => {
            let task_index = get_recent_task_index();
            let task_name = get_task_name(task_index);
            panic!(
                "Task {{ index: {task_index}, name: {task_name} }} unsupported trap {cause:?}, stval={stval_val:#x}, context={context:#x?}"
            );
        }
    }
    context
}
