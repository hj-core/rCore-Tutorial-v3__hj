use riscv::regs::{
    scause::{self, Cause},
    stval,
};

use crate::mm::prelude::{PERMISSION_R, PERMISSION_U, PERMISSION_W, check_u_va};
use crate::syscall;
use crate::task::prelude::{
    TaskState, exchange_current_task_state, get_current_task_id, record_current_run_end,
    record_current_syscall, run_next_task,
};
use crate::trap::{TrapContext, do_page_fault};
use crate::{info, log, warn};

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
