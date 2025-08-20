mod ktrap;
mod utrap;

use core::arch::{asm, global_asm};

use riscv::regs::{sie, sstatus, stvec};

use crate::mm::prelude::VMError;
use crate::task::prelude::update_tcb;

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
