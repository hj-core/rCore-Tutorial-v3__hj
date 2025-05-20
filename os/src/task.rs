pub(crate) mod loader;

use core::{
    arch::asm,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{debug, error, info, kernel_end, log, sbi::shutdown, trap::TrapContext, warn};

/// The agreed-upon address where the first user app should be installed.
const APP_BASE_PTR_0: *mut u8 = 0x8040_0000 as *mut u8;
const APP_MAX_SIZE: usize = 0x2_0000;
const APP_MAX_NUMBER: usize = 8;

const KERNEL_STACK_SIZE: usize = 0x2000; // 8KB
const USER_STACK_SIZE: usize = 0x2000; // 8KB

static mut APP_KERNEL_STACK: [KernelStack; APP_MAX_NUMBER] =
    [KernelStack([0u8; KERNEL_STACK_SIZE]); APP_MAX_NUMBER];

static mut APP_USER_STACK: [UserStack; APP_MAX_NUMBER] =
    [UserStack([0u8; USER_STACK_SIZE]); APP_MAX_NUMBER];

static NEXT_APP_INDEX: AtomicUsize = AtomicUsize::new(0);

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
    if APP_BASE_PTR_0.addr() < kernel_end as usize {
        error!("Kernel data extruded into the app-reserved addresses.");
        shutdown(true)
    }

    let failed = loader::install_all_apps();
    if failed > 0 {
        error!("{} user apps failed to install.", failed);
        shutdown(true)
    }

    if APP_MAX_NUMBER < loader::get_total_apps_found() {
        warn!(
            "{} user apps found. Supports up to {}; the rest are ignored.",
            loader::get_total_apps_found(),
            APP_MAX_NUMBER,
        );
    }

    AppRunner::run_next_app()
}

pub struct AppRunner;

impl AppRunner {
    /// `get_curr_app_index` returns the index of the currently running app.
    /// Clients should ensure that an app is indeed running; otherswis, the
    /// returned result is invalid.
    pub fn get_curr_app_index() -> usize {
        NEXT_APP_INDEX.load(Ordering::Relaxed) - 1
    }

    pub fn run_next_app() -> ! {
        let app_index = NEXT_APP_INDEX.fetch_add(1, Ordering::Relaxed);
        if app_index >= loader::get_total_apps() {
            info!("No more apps to run, bye bye.");
            shutdown(false)
        }
        Self::run_app(app_index)
    }

    fn run_app(app_index: usize) -> ! {
        assert!(
            app_index < loader::get_total_apps(),
            "Invalid app index {app_index}"
        );

        let time = Self::read_system_time_ms();
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
}
