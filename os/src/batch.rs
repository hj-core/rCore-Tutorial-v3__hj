use core::{
    arch::asm,
    slice,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::{println, sbi::shutdown, trap::TrapContext};

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
    pub fn get_init_top() -> usize {
        unsafe {
            let ptr = &raw const USER_STACK.0 as *const u8;
            ptr.add(USER_STACK_SIZE) as usize
        }
    }
}

pub struct AppManager;

impl AppManager {
    /// The agreed-upon address where the running app should be installed.
    const APP_MEM_ADDR: *mut u8 = 0x8040_0000 as *mut u8;
    const APP_MAX_SIZE: usize = 0x2_0000;
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
    ///  app_index is invalid. Only ASCII characters are supported.
    pub fn get_app_name<'a>(app_index: usize) -> &'a str {
        if app_index >= Self::get_total_apps() {
            return "";
        }
        let app_name_ptr = unsafe {
            Self::get_info_base_ptr()
                .add(app_index * AppManager::APP_META_SIZE + 1)
                .read() as *const u8
        };
        let app_name_len = Self::get_app_data_start(app_index) - (app_name_ptr as usize);
        let slice = unsafe { slice::from_raw_parts(app_name_ptr, app_name_len) };
        core::str::from_utf8(slice).unwrap()
    }

    /// `get_app_data_start` returns the starting address of the app data
    /// in the data section.
    pub fn get_app_data_start(app_index: usize) -> usize {
        if app_index >= Self::get_total_apps() {
            return 0;
        }
        unsafe {
            Self::get_info_base_ptr()
                .add(app_index * AppManager::APP_META_SIZE + 2)
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
                .add(app_index * AppManager::APP_META_SIZE + 3)
                .read() as usize
        }
    }

    pub fn run_next_app() -> ! {
        let app_index = NEXT_APP_INDEX.fetch_add(1, Ordering::Relaxed);
        if app_index >= Self::get_total_apps() {
            println!("[KERNEL] No more apps to run, bye bye.");
            shutdown(false)
        }
        Self::run_app(app_index)
    }

    fn run_app(app_index: usize) -> ! {
        println!("[KERNEL] Running {}", Self::get_app_name(app_index));
        if Self::install_app(app_index) == 0 {
            panic!("Failed to install app");
        }

        let mut kernel_sp = KernelStack::get_init_top() as *mut TrapContext;
        assert!(
            kernel_sp.is_aligned(),
            "Actions required to align the kernel_sp with TrapContext"
        );

        let init_context =
            TrapContext::new_app_context(Self::APP_MEM_ADDR as usize, UserStack::get_init_top());
        unsafe {
            kernel_sp = kernel_sp.offset(-1);
            kernel_sp.write_volatile(init_context);
        };

        unsafe extern "C" {
            unsafe fn __restore(cx: usize);
        }
        unsafe { __restore(kernel_sp as usize) };

        unreachable!()
    }

    /// `install_app` copies the app data to the agreed-upon [Self::APP_MEM_ADDR],
    /// and returns the number of bytes copied.
    fn install_app(app_index: usize) -> usize {
        if app_index >= Self::get_total_apps() {
            return 0;
        }

        let app_data_start = Self::get_app_data_start(app_index);
        let app_data_end = Self::get_app_data_end(app_index);
        let app_size = app_data_end - app_data_start;

        if app_size > Self::APP_MAX_SIZE {
            return 0;
        }

        // Clear the reserved memory range
        for i in 0..Self::APP_MAX_SIZE {
            unsafe { Self::APP_MEM_ADDR.add(i).write_volatile(0) };
        }

        // Copy the app data to the reserved memory range
        let app_data = unsafe { slice::from_raw_parts(app_data_start as *const u8, app_size) };
        let dst = unsafe { slice::from_raw_parts_mut(Self::APP_MEM_ADDR as *mut u8, app_size) };
        dst.copy_from_slice(app_data);

        unsafe { asm!("fence.i") };
        app_size
    }
}
