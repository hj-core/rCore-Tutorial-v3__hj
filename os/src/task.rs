pub(crate) mod prelude;

mod control;
mod loader;
mod runner;

use core::array;
use lazy_static::lazy_static;

use crate::{
    debug, log,
    sync::spin::SpinLock,
    task::control::{TaskControlBlock, TaskState},
};

const APP_MAX_NUMBER: usize = 8;

const KERNEL_STACK_SIZE: usize = 0x2000; // 8KB
const USER_STACK_SIZE: usize = 0x2000; // 8KB

static mut APP_KERNEL_STACK: [KernelStack; APP_MAX_NUMBER] =
    [KernelStack([0u8; KERNEL_STACK_SIZE]); APP_MAX_NUMBER];

static mut APP_USER_STACK: [UserStack; APP_MAX_NUMBER] =
    [UserStack([0u8; USER_STACK_SIZE]); APP_MAX_NUMBER];

lazy_static! {
    // An extra slot to store the switched-out context when running the first task
    static ref TASK_CONTROL_BLOCK: [SpinLock<TaskControlBlock>; APP_MAX_NUMBER + 1] =
        array::from_fn(|_| { SpinLock::new(TaskControlBlock::new_placeholder()) });
}

#[derive(Clone, Copy)]
#[repr(align(4096))]
struct KernelStack([u8; KERNEL_STACK_SIZE]);

impl KernelStack {
    fn get_upper_bound(app_index: usize) -> usize {
        unsafe {
            let ptr = &raw const APP_KERNEL_STACK[app_index].0 as *const u8;
            ptr.add(KERNEL_STACK_SIZE) as usize
        }
    }
}

#[derive(Clone, Copy)]
#[repr(align(4096))]
struct UserStack([u8; USER_STACK_SIZE]);

impl UserStack {
    fn get_upper_bound(app_index: usize) -> usize {
        unsafe {
            let ptr = &raw const APP_USER_STACK[app_index].0 as *const u8;
            ptr.add(USER_STACK_SIZE) as usize
        }
    }

    fn get_lower_bound(app_index: usize) -> usize {
        unsafe { (&raw const APP_USER_STACK[app_index].0).addr() }
    }
}

pub fn start() -> ! {
    let failed = loader::install_all_apps();
    if 0 < failed {
        panic!("{failed} user apps failed to install.");
    }

    debug_print_tcb();

    runner::init_and_run()
}

fn debug_print_tcb() {
    for i in 0..APP_MAX_NUMBER {
        let tcb = TASK_CONTROL_BLOCK[i].lock();
        debug!(
            "tcb {}: state={:?}, context={:#x?}",
            i,
            tcb.get_state(),
            tcb.get_context()
        );
    }
}

/// `exchange_recent_task_state` changes the state of the most recent task to
/// `new` if the current state is the same as `expected`.
///
/// The return value is a [Result] indicating whether the change succeeded and
/// contains the previous state.
pub(crate) fn exchange_recent_task_state(
    expected: TaskState,
    new: TaskState,
) -> Result<TaskState, TaskState> {
    let mut tcb = TASK_CONTROL_BLOCK[runner::get_recent_app_index()].lock();
    let state = tcb.get_state();

    if state == expected {
        tcb.change_state(new);
        Ok(expected)
    } else {
        Err(state)
    }
}
