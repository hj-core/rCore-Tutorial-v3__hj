use core::{
    arch::{asm, global_asm},
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{
    debug, info, log,
    sbi::shutdown,
    task::{
        KernelStack, TASK_CONTROL_BLOCK,
        control::{TaskContext, TaskState},
        debug_print_tcb,
        loader::{get_app_name, get_total_apps},
    },
    trap::{self, TrapContext},
};

global_asm!(include_str!("switch.S"));
unsafe extern "C" {
    unsafe fn __switch(curr_context: *mut TaskContext, next_context: *const TaskContext);
}

static CURRENT_APP_INDEX: AtomicUsize = AtomicUsize::new(0);

/// `get_curr_app_index` returns the index of the currently running app.
/// Clients should ensure that an app is indeed running; otherswis, the
/// returned result is invalid.
pub(crate) fn get_current_app_index() -> usize {
    CURRENT_APP_INDEX.load(Ordering::Relaxed)
}

pub(super) fn run_first_app() -> ! {
    CURRENT_APP_INDEX.store(0, Ordering::Relaxed);
    run_app(0)
}

pub(crate) fn run_next_app() -> ! {
    if let Some(app_index) = find_next_app() {
        run_app(app_index)
    } else {
        debug_print_tcb();
        info!("No more apps to run, bye bye.");
        shutdown(false)
    }
}

fn find_next_app() -> Option<usize> {
    let curr_index = get_current_app_index();
    let total_apps = get_total_apps();

    ((curr_index + 1)..(curr_index + 1 + total_apps))
        .map(|i| if i < total_apps { i } else { i - total_apps })
        .find(|&i| matches!(TASK_CONTROL_BLOCK[i].lock().get_state(), TaskState::Ready))
}

fn run_app(app_index: usize) -> ! {
    assert!(
        app_index < get_total_apps(),
        "Invalid app index {app_index}"
    );

    assert_eq!(
        TASK_CONTROL_BLOCK[app_index].lock().get_state(),
        TaskState::Ready,
        "Cannot run a non-ready app"
    );

    TASK_CONTROL_BLOCK[app_index]
        .lock()
        .change_state(TaskState::Running);

    CURRENT_APP_INDEX.store(app_index, Ordering::Relaxed);

    let time = read_system_time_ms();
    debug!(
        "{} starts at {}.{:03} seconds since system start",
        get_app_name(app_index),
        time / 1000,
        time % 1000,
    );

    let init_kernel_sp =
        unsafe { (KernelStack::get_upper_bound(app_index) as *mut TrapContext).offset(-1) };

    unsafe { trap::__restore(init_kernel_sp) };
    unreachable!()
}

/// `read_system_time_ms` returns the time since system start in millisecond.
fn read_system_time_ms() -> usize {
    let mut ticks: usize;
    unsafe { asm!("rdtime {}", out(reg) ticks) };

    const TIMER_FREQ_MHZ: usize = 10;
    ticks / (TIMER_FREQ_MHZ * 1_000)
}
