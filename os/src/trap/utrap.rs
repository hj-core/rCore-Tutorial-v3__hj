use riscv::regs::{
    scause::{self, Cause},
    sepc, stval,
};

use crate::mm::prelude::{PERMISSION_R, PERMISSION_U, PERMISSION_W, check_u_va};
use crate::syscall;
use crate::task::prelude::{
    TaskState, exchange_current_task_state, get_current_task_id, record_current_run_end,
    record_current_syscall, run_next_task,
};
use crate::trap::{TrapContext, do_page_fault, log_do_page_fault_failed, trap_panic};
use crate::{info, log, warn};

const EXPECT_RUNNING_TASK_STATE: &str = "Expected the current TaskState to be Running.";

#[unsafe(no_mangle)]
fn u_trap_handler(context: &mut TrapContext) -> &mut TrapContext {
    let task_id = get_current_task_id();
    let scause = scause::read();
    let sepc = sepc::read();
    let stval = stval::read();

    let cause = scause::match_cause(scause);
    match cause {
        Cause::SupervisorTimerInterrupt => {
            sleep_task();
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

        Cause::StoreOrAmoPageFault if check_u_va(stval) => {
            let min_permissions = PERMISSION_U | PERMISSION_W;
            if let Err(err) = do_page_fault(task_id, stval, min_permissions) {
                log_do_page_fault_failed(task_id, stval, min_permissions, err);
                kill_task();
                log_task_killed(task_id, cause, stval, sepc);
                run_next_task();
            }
        }

        Cause::LoadPageFault if check_u_va(stval) => {
            let min_permissions = PERMISSION_U | PERMISSION_R;
            if let Err(err) = do_page_fault(task_id, stval, min_permissions) {
                log_do_page_fault_failed(task_id, stval, min_permissions, err);
                kill_task();
                log_task_killed(task_id, cause, stval, sepc);
                run_next_task();
            }
        }

        Cause::IllegalInstruction => {
            kill_task();
            log_task_killed(task_id, cause, stval, sepc);
            run_next_task();
        }

        _ => trap_panic(task_id, cause, scause, stval, sepc, context),
    }

    context
}

fn sleep_task() {
    exchange_current_task_state(TaskState::Running, TaskState::Ready)
        .expect(EXPECT_RUNNING_TASK_STATE);
    record_current_run_end();
}

fn kill_task() {
    exchange_current_task_state(TaskState::Running, TaskState::Killed)
        .expect(EXPECT_RUNNING_TASK_STATE);
    record_current_run_end();
}

fn log_task_killed(task_id: usize, cause: scause::Cause, stval: usize, sepc: usize) {
    warn!(
        "Task {}: {:?}, stval={:#x}, spec={:#x}. Kernel killed it.",
        task_id, cause, stval, sepc
    );
}
