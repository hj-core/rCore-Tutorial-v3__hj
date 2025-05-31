use core::{
    arch::global_asm,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    debug, info, log,
    sbi::shutdown,
    task::{
        TASK_CONTROL_BLOCK, TASK_MAX_NUMBER,
        control::{TaskContext, TaskState},
        debug_print_tcb, get_task_name, get_total_tasks,
    },
    timer,
};

global_asm!(include_str!("switch.S"));
unsafe extern "C" {
    unsafe fn __switch(curr_context: *mut TaskContext, next_context: *const TaskContext);
}

static RECENT_TASK_INDEX: AtomicUsize = AtomicUsize::new(0);

/// `get_recent_task_index` returns the index of the most recent task.
pub(crate) fn get_recent_task_index() -> usize {
    RECENT_TASK_INDEX.load(Ordering::Relaxed)
}

/// `init_and_run` starts running the task from task 0.
pub(super) fn init_and_run() -> ! {
    RECENT_TASK_INDEX.store(TASK_MAX_NUMBER, Ordering::Relaxed);
    run_task(0);
    unreachable!()
}

/// `run_next_task` searches for and runs the next ready task starting from
/// the task following the most recent one (wrapping around if necessary).
pub(crate) fn run_next_task() {
    if let Some(task_index) = find_next_task() {
        run_task(task_index)
    } else {
        debug_print_tcb();
        info!("No more tasks to run, bye bye.");
        shutdown(false)
    }
}

/// `find_next_task` searches for the next ready task starting from the task
/// following the most recent one (wrapping around if necessary) and returns
/// its index.
fn find_next_task() -> Option<usize> {
    let curr_index = get_recent_task_index();
    let total_tasks = get_total_tasks();

    ((curr_index + 1)..(curr_index + 1 + total_tasks))
        .map(|i| if i < total_tasks { i } else { i - total_tasks })
        .find(|&i| matches!(TASK_CONTROL_BLOCK[i].lock().get_state(), TaskState::Ready))
}

fn run_task(task_index: usize) {
    assert!(
        task_index < get_total_tasks(),
        "Invalid task index {task_index}"
    );

    let curr_context = TASK_CONTROL_BLOCK[get_recent_task_index()]
        .lock()
        .get_mut_context() as *mut TaskContext;

    let mut next_tcb = TASK_CONTROL_BLOCK[task_index].lock();
    assert_eq!(
        next_tcb.get_state(),
        TaskState::Ready,
        "Attempted to run a non-ready Task {{ index: {}, name: {} }}",
        task_index,
        get_task_name(task_index)
    );

    next_tcb.change_state(TaskState::Running);
    let next_context = next_tcb.get_context() as *const TaskContext;

    RECENT_TASK_INDEX.store(task_index, Ordering::Relaxed);

    let time = timer::read_time_ms();
    debug!(
        "Task {{ index: {}, name: {} }} starts at {}.{:03} seconds since system start",
        task_index,
        get_task_name(task_index),
        time / 1000,
        time % 1000,
    );

    next_tcb.record_run_start();
    drop(next_tcb);
    timer::set_next_timer_interrupt();
    unsafe { __switch(curr_context, next_context) };
}
