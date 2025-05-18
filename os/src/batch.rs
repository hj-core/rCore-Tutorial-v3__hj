use core::{
    arch::asm,
    slice,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{debug, error, info, kernel_end, log, sbi::shutdown, trap::TrapContext, warn};

/// The agreed-upon address where the first user app should be installed.
const APP_BASE_PTR_0: *mut u8 = 0x8040_0000 as *mut u8;
const APP_MAX_SIZE: usize = 0x2_0000;

const KERNEL_STACK_SIZE: usize = 0x2000; // 8KB
const USER_STACK_SIZE: usize = 0x2000; // 8KB

static mut KERNEL_STACK: KernelStack = KernelStack([0u8; KERNEL_STACK_SIZE]);
static mut USER_STACK: UserStack = UserStack([0u8; USER_STACK_SIZE]);

static NEXT_APP_INDEX: AtomicUsize = AtomicUsize::new(0);

#[repr(align(4096))]
struct KernelStack([u8; KERNEL_STACK_SIZE]);

impl KernelStack {
    fn get_init_top() -> usize {
        unsafe {
            let ptr = &raw const KERNEL_STACK.0 as *const u8;
            ptr.add(KERNEL_STACK_SIZE) as usize
        }
    }
}

#[repr(align(4096))]
struct UserStack([u8; USER_STACK_SIZE]);

impl UserStack {
    fn get_init_top() -> usize {
        unsafe {
            let ptr = &raw const USER_STACK.0 as *const u8;
            ptr.add(USER_STACK_SIZE) as usize
        }
    }

    fn get_lower_bound() -> usize {
        unsafe { (&raw const USER_STACK.0).addr() }
    }
}

pub fn start() -> ! {
    if APP_BASE_PTR_0.addr() < kernel_end as usize {
        error!("Kernel data extruded into the app-reserved addresses.");
        shutdown(true)
    }

    let failed = AppLoader::install_all_apps();
    if failed > 0 {
        error!("{} user apps failed to install.", failed);
        shutdown(true)
    }

    AppRunner::run_next_app()
}

pub struct AppLoader;

impl AppLoader {
    /// The number of meta information items kept for each app.
    ///
    /// Currently, we keep app_name, app_start, and app_end for each app under
    /// the _num_apps in the generated link_apps.S.
    const APP_META_SIZE: usize = 3;

    /// `get_info_base_ptr` returns a pointer to the _num_apps, which is
    /// defined in the generated link_app.S.
    fn get_info_base_ptr() -> *const u64 {
        unsafe extern "C" {
            unsafe fn _num_apps();
        }
        _num_apps as *const u64
    }

    /// `get_total_apps` returns the total number of user apps.
    pub fn get_total_apps() -> usize {
        unsafe { Self::get_info_base_ptr().read() as usize }
    }

    /// `get_app_name` returns the name of the app, or an empty string if the
    /// app index is invalid or the app name is invalid. Only ASCII characters
    /// are supported.
    pub fn get_app_name<'a>(app_index: usize) -> &'a str {
        if app_index >= Self::get_total_apps() {
            return "";
        }
        let app_name_ptr = unsafe {
            Self::get_info_base_ptr()
                .add(app_index * Self::APP_META_SIZE + 1)
                .read() as *const u8
        };
        let app_name_len = Self::get_app_data_start(app_index) - (app_name_ptr as usize);
        let slice = unsafe { slice::from_raw_parts(app_name_ptr, app_name_len) };
        core::str::from_utf8(slice).unwrap_or("")
    }

    /// `get_app_data_start` returns the starting address of the app data
    /// in the data section.
    pub fn get_app_data_start(app_index: usize) -> usize {
        if app_index >= Self::get_total_apps() {
            return 0;
        }
        unsafe {
            Self::get_info_base_ptr()
                .add(app_index * Self::APP_META_SIZE + 2)
                .read() as usize
        }
    }

    /// `get_app_data_end` returns the end address (exclusive) of the app
    /// data in the data section.
    pub fn get_app_data_end(app_index: usize) -> usize {
        if app_index >= Self::get_total_apps() {
            return 0;
        }
        unsafe {
            Self::get_info_base_ptr()
                .add(app_index * Self::APP_META_SIZE + 3)
                .read() as usize
        }
    }

    /// `get_app_base_ptr` returns a pointer to the agreed-upon memory address
    /// where the app should be installed.
    pub fn get_app_base_ptr(app_index: usize) -> *mut u8 {
        unsafe { APP_BASE_PTR_0.add(app_index * APP_MAX_SIZE) }
    }

    /// `install_all_apps` copies all user app data to the agreed-upon memory
    /// addresses and returns the number of failed installations.
    fn install_all_apps() -> usize {
        let mut result = 0;
        for app_index in 0..Self::get_total_apps() {
            if Self::install_app(app_index) == 0 {
                warn!(
                    "Failed to install user app: index={}, name={}",
                    app_index,
                    Self::get_app_name(app_index)
                );
                result += 1;
            }
        }

        // Prevent CPU from using outdated instruction cache
        unsafe { asm!("fence.i") };

        result
    }

    /// `install_app` copies the app data to the agreed-upon memory address and
    /// returns the number of bytes copied.
    fn install_app(app_index: usize) -> usize {
        if app_index >= Self::get_total_apps() {
            return 0;
        }

        let app_data_start = Self::get_app_data_start(app_index);
        let app_data_end = Self::get_app_data_end(app_index);
        let app_size = app_data_end - app_data_start;

        if app_size > APP_MAX_SIZE {
            return 0;
        }

        // Clear the reserved memory range
        let app_base_ptr = Self::get_app_base_ptr(app_index);
        for i in 0..APP_MAX_SIZE {
            unsafe { app_base_ptr.add(i).write_volatile(0) };
        }

        // Copy the app data to the reserved memory range
        let app_data = unsafe { slice::from_raw_parts(app_data_start as *const u8, app_size) };
        let dst = unsafe { slice::from_raw_parts_mut(app_base_ptr, app_size) };
        dst.copy_from_slice(app_data);

        app_size
    }

    /// `can_app_read_addr` returns whether the address is readable by the currently running
    /// app. Clients should ensure that an app is indeed running; otherswis, the returned
    /// result is invalid.
    pub fn can_app_read_addr(app_index: usize, addr: usize) -> bool {
        let app_size = Self::get_app_data_end(app_index) - Self::get_app_data_start(app_index);
        if app_size == 0 {
            return false;
        }

        let app_base_addr = Self::get_app_base_ptr(app_index).addr();
        let data_range = app_base_addr..(app_base_addr + app_size);
        if data_range.contains(&addr) {
            return true;
        }

        let stack_range = UserStack::get_lower_bound()..UserStack::get_init_top();
        if stack_range.contains(&addr) {
            return true;
        }

        false
    }
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
        if app_index >= AppLoader::get_total_apps() {
            info!("No more apps to run, bye bye.");
            shutdown(false)
        }
        Self::run_app(app_index)
    }

    fn run_app(app_index: usize) -> ! {
        assert!(
            app_index < AppLoader::get_total_apps(),
            "Invalid app index {app_index}"
        );

        let mut kernel_sp = KernelStack::get_init_top() as *mut TrapContext;
        assert!(
            kernel_sp.is_aligned(),
            "Actions required to align the kernel_sp with TrapContext"
        );

        let app_base_addr = AppLoader::get_app_base_ptr(app_index).addr();
        let init_context = TrapContext::new_app_context(app_base_addr, UserStack::get_init_top());
        unsafe {
            kernel_sp = kernel_sp.offset(-1);
            kernel_sp.write_volatile(init_context);
        };

        let time = Self::read_system_time_ms();
        debug!(
            "{} starts at {}.{:03} seconds since system start",
            AppLoader::get_app_name(app_index),
            time / 1000,
            time % 1000,
        );

        unsafe extern "C" {
            unsafe fn __restore(cx: usize);
        }
        unsafe { __restore(kernel_sp as usize) };

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
