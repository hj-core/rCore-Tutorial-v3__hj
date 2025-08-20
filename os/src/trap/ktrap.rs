use riscv::regs::{
    scause::{self, Cause},
    sepc, stval,
};

use crate::mm::prelude::{
    PERMISSION_R, PERMISSION_U, PERMISSION_W, get_uaccess_fix, is_load_user_fault,
    is_store_user_fault,
};
use crate::trap::{TrapContext, do_page_fault};
use crate::{log, warn};

#[unsafe(no_mangle)]
fn k_trap_handler(context: &mut TrapContext) {
    let scause_val = scause::read();
    let cause = scause::match_cause(scause_val);
    let sepc = sepc::read();
    let stval_val = stval::read();
    let saved_tp = context.x[4];

    match cause {
        Cause::LoadPageFault if saved_tp != 0 && is_load_user_fault(sepc) => {
            // SAFETY:
            // If the saved_tp is not zero, it should point to the
            // [TrapContext] of the running task on this hart.
            let task_id = unsafe { TrapContext::get_task_id_from_ptr(saved_tp as *const _) };

            if let Err(err) = do_page_fault(task_id, stval_val, PERMISSION_U | PERMISSION_R) {
                warn!(
                    "Task {:}: Mapping the faulted page from uaccess failed with {:?}.",
                    task_id, err
                );
                context.sepc = get_uaccess_fix();
            }
        }

        Cause::StoreOrAmoPageFault if saved_tp != 0 && is_store_user_fault(sepc) => {
            // SAFETY:
            // If the saved_tp is not zero, it should point to the
            // [TrapContext] of the running task on this hart.
            let task_id = unsafe { TrapContext::get_task_id_from_ptr(saved_tp as *const _) };

            if let Err(err) = do_page_fault(task_id, stval_val, PERMISSION_U | PERMISSION_W) {
                warn!(
                    "Task {:}: Mapping the faulted page from uaccess failed with {:?}.",
                    task_id, err
                );
                context.sepc = get_uaccess_fix();
            }
        }

        _ => {
            panic!(
                "Kernel trapped by {cause:?}, sepc={sepc:#x}, scause={scause_val:#x}, stval={stval_val:#x}"
            )
        }
    }
}
