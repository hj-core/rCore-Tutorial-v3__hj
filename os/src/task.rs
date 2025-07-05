pub(crate) mod prelude;

mod control;
mod loader;
mod runner;

use core::{array, cmp::min};
use lazy_static::lazy_static;

use crate::{
    debug, log,
    sync::spin::SpinLock,
    task::control::{TaskControlBlock, TaskState, TaskStatistics},
    trap::{self, TrapContext},
};

const TASK_MAX_NUMBER: usize = 8;
const KERNEL_STACK_SIZE: usize = 0x2000; // 8KB
const USER_STACK_SIZE: usize = 0x2000; // 8KB

static mut TASK_KERNEL_STACK: [KernelStack; TASK_MAX_NUMBER] =
    [KernelStack([0u8; KERNEL_STACK_SIZE]); TASK_MAX_NUMBER];

static mut TASK_USER_STACK: [UserStack; TASK_MAX_NUMBER] =
    [UserStack([0u8; USER_STACK_SIZE]); TASK_MAX_NUMBER];

lazy_static! {
    // An extra slot to store the switched-out context when running the first task
    static ref TASK_CONTROL_BLOCK: [SpinLock<TaskControlBlock>; TASK_MAX_NUMBER + 1] =
        array::from_fn(|_| { SpinLock::new(TaskControlBlock::new_placeholder()) });
}

#[derive(Clone, Copy)]
#[repr(align(4096))]
struct KernelStack([u8; KERNEL_STACK_SIZE]);

impl KernelStack {
    fn get_upper_bound(task_index: usize) -> usize {
        unsafe {
            let ptr = &raw const TASK_KERNEL_STACK[task_index].0 as *const u8;
            ptr.add(KERNEL_STACK_SIZE) as usize
        }
    }
}

#[derive(Clone, Copy)]
#[repr(align(4096))]
struct UserStack([u8; USER_STACK_SIZE]);

impl UserStack {
    fn get_upper_bound(task_index: usize) -> usize {
        unsafe {
            let ptr = &raw const TASK_USER_STACK[task_index].0 as *const u8;
            ptr.add(USER_STACK_SIZE) as usize
        }
    }

    fn get_lower_bound(task_index: usize) -> usize {
        unsafe { (&raw const TASK_USER_STACK[task_index].0).addr() }
    }
}

pub(super) fn start() -> ! {
    push_all_first_run_trap_contexts();
    set_all_first_run_tcbs();

    debug_print_tcb();

    runner::init_and_run()
}

fn get_total_tasks() -> usize {
    min(TASK_MAX_NUMBER, loader::get_total_apps())
}

pub(crate) fn get_task_name<'a>(task_index: usize) -> &'a str {
    loader::get_app_name(task_index)
}

/// `push_all_first_run_trap_contexts` pushes the first run trap context onto the
/// task's kernel stack for each task.
fn push_all_first_run_trap_contexts() {
    for task_index in 0..get_total_tasks() {
        push_first_run_trap_context(task_index);
    }
}

/// `push_first_run_trap_context` pushes the first run trap context onto the task's
/// kernel stack.
fn push_first_run_trap_context(task_index: usize) {
    let init_context = TrapContext::new_init_context(
        get_task_entry_addr(task_index),
        UserStack::get_upper_bound(task_index),
    );

    let mut kernel_sp = KernelStack::get_upper_bound(task_index) as *mut TrapContext;
    assert!(
        kernel_sp.is_aligned(),
        "Actions required to align the kernel_sp with TrapContext"
    );
    unsafe {
        kernel_sp = kernel_sp.offset(-1);
        kernel_sp.write_volatile(init_context);
    };
}

fn get_task_entry_addr(task_index: usize) -> usize {
    loader::get_app_entry_ptr(task_index).addr()
}

/// `set_all_first_run_tcbs` configures the [TaskControlBlock] for each task's
/// first run.
///
/// [TaskControlBlock]: super::control::TaskControlBlock
fn set_all_first_run_tcbs() {
    for task_index in 0..get_total_tasks() {
        set_first_run_tcb(task_index);
    }
}

/// `set_first_run_tcb` configures the [TaskControlBlock] of the task for its
/// first run.
///
/// [TaskControlBlock]: super::control::TaskControlBlock
fn set_first_run_tcb(task_index: usize) {
    let mut tcb = TASK_CONTROL_BLOCK[task_index].lock();
    tcb.change_state(TaskState::Ready);

    let init_kernel_sp = KernelStack::get_upper_bound(task_index) - size_of::<TrapContext>();
    let context = tcb.get_mut_context();
    context.set_ra(trap::__restore as usize);
    context.set_sp(init_kernel_sp);
}

fn debug_print_tcb() {
    for i in 0..TASK_MAX_NUMBER {
        let tcb = TASK_CONTROL_BLOCK[i].lock();
        debug!("TCB {}: {:#?}", i, tcb);
    }
}

pub(crate) fn can_task_read_addr(task_index: usize, addr: usize) -> bool {
    in_task_user_stack(task_index, addr) || loader::is_app_installed_data(task_index, addr)
}

fn in_task_user_stack(task_index: usize, addr: usize) -> bool {
    let lower_bound = UserStack::get_lower_bound(task_index);
    let upper_bound = UserStack::get_upper_bound(task_index);
    lower_bound <= addr && addr < upper_bound
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
    let mut tcb = TASK_CONTROL_BLOCK[runner::get_recent_task_index()].lock();
    let state = tcb.get_state();

    if state == expected {
        tcb.change_state(new);
        Ok(expected)
    } else {
        Err(state)
    }
}

/// `record_syscall_for_recent_task` records a call to the syscall for the
/// recent task, which may fail since only the first [MAX_SYSCALLS_TRACKED]
/// different syscalls are tracked. It returns a boolean indicating whether
/// the call has been recorded.
///
/// [MAX_SYSCALLS_TRACKED]: control::MAX_SYSCALLS_TRACKED
pub(crate) fn record_syscall_for_recent_task(syscall_id: usize) -> bool {
    TASK_CONTROL_BLOCK[runner::get_recent_task_index()]
        .lock()
        .record_syscall(syscall_id)
}

pub(crate) fn get_task_info(task_index: usize) -> TaskInfo {
    let task_index = min(task_index, TASK_MAX_NUMBER);
    let tcb = TASK_CONTROL_BLOCK[task_index].lock();
    TaskInfo {
        task_id: task_index,
        state: tcb.get_state(),
        stastics: tcb.get_statistics(),
    }
}

#[allow(dead_code)]
pub(crate) struct TaskInfo {
    task_id: usize,
    state: TaskState,
    stastics: TaskStatistics,
}
