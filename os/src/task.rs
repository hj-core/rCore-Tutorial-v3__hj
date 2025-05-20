pub(crate) mod loader;
pub(crate) mod runner;

use crate::{error, kernel_end, log, sbi::shutdown, warn};

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

    runner::run_first_app()
}
