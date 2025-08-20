use riscv::regs::{
    scause::{self, Cause},
    sepc, stval,
};

use crate::mm::prelude::{
    PERMISSION_R, PERMISSION_U, PERMISSION_W, get_uaccess_fix, is_load_user_fault,
    is_store_user_fault,
};
use crate::trap::{TrapContext, do_page_fault, log_do_page_fault_failed, trap_panic};

#[unsafe(no_mangle)]
fn k_trap_handler(context: &mut TrapContext) {
    let scause = scause::read();
    let sepc = sepc::read();
    let stval = stval::read();
    let saved_tp = context.x[4];

    let cause = scause::match_cause(scause);
    match cause {
        Cause::LoadPageFault if saved_tp != 0 && is_load_user_fault(sepc) => {
            // SAFETY:
            // If the saved_tp is not zero, it should point to the
            // [TrapContext] of the running task on this hart.
            let task_id = unsafe { TrapContext::get_task_id_from_ptr(saved_tp as *const _) };

            let min_permissions = PERMISSION_U | PERMISSION_R;
            if let Err(err) = do_page_fault(task_id, stval, min_permissions) {
                log_do_page_fault_failed(task_id, stval, min_permissions, err);
                context.sepc = get_uaccess_fix();
            }
        }

        Cause::StoreOrAmoPageFault if saved_tp != 0 && is_store_user_fault(sepc) => {
            // SAFETY:
            // If the saved_tp is not zero, it should point to the
            // [TrapContext] of the running task on this hart.
            let task_id = unsafe { TrapContext::get_task_id_from_ptr(saved_tp as *const _) };

            let min_permissions = PERMISSION_U | PERMISSION_W;
            if let Err(err) = do_page_fault(task_id, stval, min_permissions) {
                log_do_page_fault_failed(task_id, stval, min_permissions, err);
                context.sepc = get_uaccess_fix();
            }
        }

        _ => trap_panic(0, cause, scause, stval, sepc, context),
    }
}
