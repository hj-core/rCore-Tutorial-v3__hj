extern crate alloc;

mod apps;
pub(crate) mod prelude;
mod state;

use alloc::vec::Vec;
use core::arch::{asm, global_asm};
use core::ptr::null_mut;

use crate::mm::prelude::{VMError, VMSpace};
use crate::sbi::shutdown;
use crate::sync::spin::SpinLock;
use crate::timer;
use crate::trap::{self, TrapContext};
use crate::{debug, info, log};

use crate::task::apps::{get_app_elf, get_total_apps};
use crate::task::state::{TaskContext, TaskControlBlock, TaskState, TaskStatistics};

// The design should be revisited if the environment
// is not single-threaded, not single-core, or allows
// interrupts when the kernel is running.
static ALL_TASKS: SpinLock<Vec<TaskControlBlock>> = SpinLock::new(Vec::new());

global_asm!(include_str!("task/switch.S"));
unsafe extern "C" {
    unsafe fn __switch(curr_context: *mut TaskContext, next_context: *const TaskContext);
}

pub(super) fn start() -> ! {
    for i in 0..get_total_apps() {
        add_task(get_app_elf(i));
    }
    run_next_task();
    unreachable!()
}

fn add_task(elf_bytes: &[u8]) {
    // Create task vm space and tcb
    let vm_space = VMSpace::new_user(elf_bytes).expect("Failed to create user vm space");
    let satp = vm_space.get_satp();
    let entry = vm_space.get_entry_addr();
    let user_sp = vm_space.get_u_stack_end();
    let kernel_sp = vm_space.get_k_stack_end() - size_of::<TrapContext>();

    let tcb = TaskControlBlock::new_ready(
        vm_space,
        trap::__restore_u_ctx as usize,
        kernel_sp,
        kernel_sp,
        satp,
    );

    // Push initial trap context to kernel stack
    let trap_context = TrapContext::new_initial(entry, user_sp, tcb.get_task_id());
    unsafe { (kernel_sp as *mut TrapContext).write_volatile(trap_context) };

    // Push tcb to the task list
    ALL_TASKS.lock().push(tcb);
}

/// Searches for and runs a ready task, or shuts down
/// if no task is found.
pub(crate) fn run_next_task() {
    let mut all_tasks = ALL_TASKS.lock();

    let mut curr_context = null_mut();
    if let Some(task_id) = get_current_task_id()
        && let Some(mut tcb) = take_task_tcb(&mut all_tasks, task_id)
    {
        curr_context = tcb.get_context_mut() as *mut TaskContext;
        let state = tcb.get_state();
        if state == TaskState::Ready {
            all_tasks.push(tcb);
        } else if state == TaskState::Running {
            panic!("Attempt to switch task {task_id} but its state is running")
        }
    }

    if let Some(task_id) = get_next_task_id(&all_tasks) {
        let mut tcb = take_task_tcb(&mut all_tasks, task_id).unwrap();
        let next_context = tcb.get_context() as *const TaskContext;

        let time = timer::read_time_ms();
        debug!(
            "Task {} starts at {}.{:03} seconds since system start",
            task_id,
            time / 1000,
            time % 1000,
        );
        tcb.set_state(TaskState::Running);
        tcb.record_run_start();
        all_tasks.push(tcb);

        // The design should be revisited if the environment
        // is not single-threaded, not single-core, or allows
        // interrupts when the kernel is running.
        drop(all_tasks);

        timer::set_next_timer_interrupt();
        unsafe { __switch(curr_context, next_context) };
    } else {
        info!("No more tasks to run, bye bye.");
        shutdown(false)
    }
}

fn get_next_task_id(tasks: &Vec<TaskControlBlock>) -> Option<usize> {
    tasks
        .iter()
        .position(|tcb| tcb.get_state() == TaskState::Ready)
        .map(|i| tasks[i].get_task_id())
}

/// Returns the task ID of the current task based
/// on the current thread pointer, i.e., tp.
pub(crate) fn get_current_task_id() -> Option<usize> {
    let mut tp: usize;
    unsafe { asm!("mv {}, tp", out(reg) tp) };

    if tp == 0 {
        return None;
    }
    let task_id = unsafe { (tp as *const TrapContext).as_ref() }
        .unwrap()
        .get_task_id();
    Some(task_id)
}

/// Takes the [TaskControlBlock] with `task_id` from
/// `tasks`, or returns [None] if no matching task is
/// found.
fn take_task_tcb(tasks: &mut Vec<TaskControlBlock>, task_id: usize) -> Option<TaskControlBlock> {
    let index = tasks.iter().position(|tcb| tcb.get_task_id() == task_id)?;
    let rand_index = timer::read_time() % tasks.len();
    tasks.swap(index, rand_index);
    Some(tasks.swap_remove(rand_index))
}

/// Tries to fix the page fault for the task by mapping
/// the page containing address `stval` into its [VMSpace].
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

/// Updates the [TaskControlBlock] associated with `task_id`
/// by applying the function `f`.
///
/// # Panic
/// This function panics if no matching [TaskControlBlock]
/// can be found.
fn update_tcb(task_id: usize, f: impl FnOnce(&mut TaskControlBlock)) {
    ALL_TASKS
        .lock()
        .iter_mut()
        .find(|tcb| tcb.get_task_id() == task_id)
        .map(f)
        .expect("Cannot find a task with the task_id");
}

/// Changes the state of the current task to `new` if
/// the current state is the same as `expected`.
///
/// The return value is a [Result] indicating whether
/// the change succeeded and contains the previous state.
///
/// This function panics if the thread is not running
/// a task.
pub(crate) fn exchange_current_task_state(
    expected: TaskState,
    new: TaskState,
) -> Result<TaskState, TaskState> {
    let task_id = get_current_task_id().expect("No current task running in this thread.");

    let mut all_tasks = ALL_TASKS.lock();
    let tcb = all_tasks
        .iter_mut()
        .find(|tcb| tcb.get_task_id() == task_id)
        .unwrap();
    let state = tcb.get_state();

    if state == expected {
        tcb.set_state(new);
        Ok(expected)
    } else {
        Err(state)
    }
}

/// Records the current mtime as the current task's
/// last run end time, and updates the total executed
/// time and the switch count.
///
/// This function panics if the thread is not running
/// a task.
pub(crate) fn record_current_run_end() {
    let task_id = get_current_task_id().expect("No current task running in this thread.");

    let mut all_tasks = ALL_TASKS.lock();
    let tcb = all_tasks
        .iter_mut()
        .find(|tcb| tcb.get_task_id() == task_id)
        .unwrap();
    tcb.record_run_end();
}

/// Records a call to the syscall for the recent task,
/// which may fail since only the first [MAX_SYSCALLS_TRACKED]
/// different syscalls are tracked. It returns a boolean
/// indicating whether the call has been recorded.
///
/// This function panics if the thread is not running
/// a task.
///
/// [MAX_SYSCALLS_TRACKED]: control::MAX_SYSCALLS_TRACKED
pub(crate) fn record_current_syscall(syscall_id: usize) -> bool {
    let task_id = get_current_task_id().expect("No current task running in this thread.");

    let mut all_tasks = ALL_TASKS.lock();
    let tcb = all_tasks
        .iter_mut()
        .find(|tcb| tcb.get_task_id() == task_id)
        .unwrap();
    tcb.record_syscall(syscall_id)
}

/// Returns the [TaskInfo] of the task with `task_id`,
/// or [None] if no matching task is found
pub(crate) fn get_task_info(task_id: usize) -> Option<TaskInfo> {
    let all_tasks = ALL_TASKS.lock();
    let tcb = all_tasks.iter().find(|tcb| tcb.get_task_id() == task_id)?;

    Some(TaskInfo {
        task_id,
        state: tcb.get_state(),
        stastics: tcb.get_statistics(),
    })
}

#[allow(dead_code)]
pub(crate) struct TaskInfo {
    task_id: usize,
    state: TaskState,
    stastics: TaskStatistics,
}
