mod ktrap;

use core::arch::{asm, global_asm};

use riscv::regs::{
    scause::{self, Cause},
    sie, sstatus, stval, stvec,
};

use crate::mm::prelude::{PERMISSION_R, PERMISSION_U, PERMISSION_W, VMError, check_u_va};
use crate::syscall;
use crate::task::prelude::{
    TaskState, exchange_current_task_state, get_current_task_id, record_current_run_end,
    record_current_syscall, run_next_task, update_tcb,
};
use crate::{info, log, warn};

global_asm!(include_str!("trap.S"));

unsafe extern "C" {
    // Defined in trap.S
    pub(super) unsafe fn __restore_u_ctx();
}

pub fn init() {
    unsafe extern "C" {
        unsafe fn __stvec();
    }

    // Set sscrtach to 0, indicating that we are presently
    // in kernel.
    unsafe { asm!("csrw sscratch, x0") };

    let stvec_ok = stvec::install(__stvec as usize, stvec::Mode::Direct);
    assert!(stvec_ok, "Failed to install stvec");

    enable_interrupts();
    enable_timer_interrupts();
}

/// Enables all interrupts in supervisor mode. This
/// provides overall control over interrupt behavior.
fn enable_interrupts() {
    sstatus::set_sie();
}

/// Enables the timer interrupts in supervisor mode.
/// This provides fine control over interrupt behavior.
fn enable_timer_interrupts() {
    sie::set_stie();
}

/// Tries to fix the page fault for the task by mapping the
/// page containing address `stval` into its [VMSpace].
pub(crate) fn do_page_fault(
    task_id: usize,
    stval: usize,
    min_permissions: usize,
) -> Result<(), VMError> {
    let mut result = Ok(());
    update_tcb(task_id, |tcb| {
        result = tcb
            .get_vm_space_mut()
            .map_fault_page(stval, min_permissions);
    });
    result
}

#[unsafe(no_mangle)]
fn u_trap_handler(context: &mut TrapContext) -> &mut TrapContext {
    let task_id = get_current_task_id()
        .expect("Expect the hart is running a task when entering u_trap_handler");
    let scause_val = scause::read();
    let cause = scause::match_cause(scause_val);
    let stval_val = stval::read();

    match cause {
        Cause::SupervisorTimerInterrupt => {
            exchange_current_task_state(TaskState::Running, TaskState::Ready)
                .expect("Expected the current TaskState to be Running");
            record_current_run_end();

            info!("Task {:?}: {:?}.", task_id, cause);
            run_next_task();
        }

        Cause::UserEnvironmentCall => {
            let syscall_id = context.x[17];
            record_current_syscall(syscall_id);

            context.sepc += 4;
            context.x[10] =
                syscall::syscall_handler(syscall_id, [context.x[10], context.x[11], context.x[12]])
                    as usize;
        }

        Cause::StoreOrAmoPageFault if check_u_va(stval_val) => {
            if let Err(err) = do_page_fault(task_id, stval_val, PERMISSION_U | PERMISSION_W) {
                exchange_current_task_state(TaskState::Running, TaskState::Killed)
                    .expect("Expected the current TaskState to be Running");
                record_current_run_end();

                warn!(
                    "Task {:?}: {:?}, stval={:#x}, sepc={:#x}. Mapping the faulted page failed with {:?}. Kernel killed it.",
                    task_id, cause, stval_val, context.sepc, err
                );
                run_next_task();
            }
        }

        Cause::LoadPageFault if check_u_va(stval_val) => {
            if let Err(err) = do_page_fault(task_id, stval_val, PERMISSION_U | PERMISSION_R) {
                exchange_current_task_state(TaskState::Running, TaskState::Killed)
                    .expect("Expected the current TaskState to be Running");
                record_current_run_end();

                warn!(
                    "Task {:?}: {:?}, stval={:#x}, sepc={:#x}. Mapping the faulted page failed with {:?}. Kernel killed it.",
                    task_id, cause, stval_val, context.sepc, err
                );
                run_next_task();
            }
        }

        Cause::IllegalInstruction => {
            exchange_current_task_state(TaskState::Running, TaskState::Killed)
                .expect("Expected the current TaskState to be Running");
            record_current_run_end();

            warn!("Task {:?}: {:?}, kernel killed it.", task_id, cause);
            run_next_task();
        }

        _ => {
            panic!(
                "Task {task_id:?}: {cause:?}, scause={scause_val:#x}, stval={stval_val:#x}, context={context:#x?}",
            );
        }
    }

    context
}

#[derive(Debug)]
#[repr(C)]
pub struct TrapContext {
    /// Stores the values of registers x0 through x31.
    x: [usize; 32],
    sstatus: usize,
    sepc: usize,
    task_id: usize,
}

impl TrapContext {
    pub(crate) fn new_initial(entry_addr: usize, user_sp: usize, task_id: usize) -> Self {
        let sstatus = sstatus::set_spp_user();

        let mut result = Self {
            x: [0; 32],
            sstatus,
            sepc: entry_addr,
            task_id,
        };
        result.x[2] = user_sp;
        result
    }

    pub(crate) fn get_task_id(&self) -> usize {
        self.task_id
    }

    /// Returns the `task_id` of the [TrapContext].
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [TrapContext].
    unsafe fn get_task_id_from_ptr(ptr: *const TrapContext) -> usize {
        unsafe { ptr.as_ref().unwrap().get_task_id() }
    }
}
