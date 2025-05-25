use core::{
    arch::{asm, global_asm},
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    debug, info, log,
    sbi::shutdown,
    task::{
        APP_MAX_NUMBER, TASK_CONTROL_BLOCK,
        control::{TaskContext, TaskState},
        debug_print_tcb,
        loader::{get_app_name, get_total_apps},
    },
};

global_asm!(include_str!("switch.S"));
unsafe extern "C" {
    unsafe fn __switch(curr_context: *mut TaskContext, next_context: *const TaskContext);
}

static RECENT_APP_INDEX: AtomicUsize = AtomicUsize::new(0);

/// `get_recent_app_index` returns the most recent index run by the runner.
pub(crate) fn get_recent_app_index() -> usize {
    RECENT_APP_INDEX.load(Ordering::Relaxed)
}

/// `init_and_run` starts running the apps from app 0.
pub(super) fn init_and_run() -> ! {
    RECENT_APP_INDEX.store(APP_MAX_NUMBER, Ordering::Relaxed);
    run_app(0);
    unreachable!()
}

pub(crate) fn run_next_app() {
    if let Some(app_index) = find_next_app() {
        run_app(app_index)
    } else {
        debug_print_tcb();
        info!("No more apps to run, bye bye.");
        shutdown(false)
    }
}

fn find_next_app() -> Option<usize> {
    let curr_index = get_recent_app_index();
    let total_apps = get_total_apps();

    ((curr_index + 1)..(curr_index + 1 + total_apps))
        .map(|i| if i < total_apps { i } else { i - total_apps })
        .find(|&i| matches!(TASK_CONTROL_BLOCK[i].lock().get_state(), TaskState::Ready))
}

fn run_app(app_index: usize) {
    assert!(
        app_index < get_total_apps(),
        "Invalid app index {app_index}"
    );

    let mut next_tcb = TASK_CONTROL_BLOCK[app_index].lock();
    assert_eq!(
        next_tcb.get_state(),
        TaskState::Ready,
        "Cannot run a non-ready app"
    );
    next_tcb.change_state(TaskState::Running);

    let next_context = next_tcb.get_context() as *const TaskContext;
    drop(next_tcb);

    let curr_context = TASK_CONTROL_BLOCK[get_recent_app_index()]
        .lock()
        .get_mut_context() as *mut TaskContext;

    RECENT_APP_INDEX.store(app_index, Ordering::Relaxed);

    let time = read_system_time_ms();
    debug!(
        "{} starts at {}.{:03} seconds since system start",
        get_app_name(app_index),
        time / 1000,
        time % 1000,
    );

    unsafe { __switch(curr_context, next_context) };
}

/// `read_system_time_ms` returns the time since system start in millisecond.
fn read_system_time_ms() -> usize {
    let mut ticks: usize;
    unsafe { asm!("rdtime {}", out(reg) ticks) };

    const TIMER_FREQ_MHZ: usize = 10;
    ticks / (TIMER_FREQ_MHZ * 1_000)
}
