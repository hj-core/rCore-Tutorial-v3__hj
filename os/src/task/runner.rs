use core::{
    arch::asm,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{debug, info, log, sbi::shutdown, trap::TrapContext};

use super::{KernelStack, loader};

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
    let app_index = CURRENT_APP_INDEX.fetch_add(1, Ordering::Relaxed) + 1;
    if app_index == loader::get_total_apps() {
        info!("No more apps to run, bye bye.");
        shutdown(false)
    }
    run_app(app_index)
}

fn run_app(app_index: usize) -> ! {
    assert!(
        app_index < loader::get_total_apps(),
        "Invalid app index {app_index}"
    );

    let time = read_system_time_ms();
    debug!(
        "{} starts at {}.{:03} seconds since system start",
        loader::get_app_name(app_index),
        time / 1000,
        time % 1000,
    );

    let init_kernel_sp =
        unsafe { (KernelStack::get_upper_bound(app_index) as *mut TrapContext).offset(-1) };

    unsafe extern "C" {
        unsafe fn __restore(cx: usize);
    }
    unsafe { __restore(init_kernel_sp as usize) };

    unreachable!()
}

/// `read_system_time_ms` returns the time since system start in millisecond.
fn read_system_time_ms() -> usize {
    let mut ticks: usize;
    unsafe { asm!("rdtime {}", out(reg) ticks) };

    const TIMER_FREQ_MHZ: usize = 10;
    ticks / (TIMER_FREQ_MHZ * 1_000)
}
