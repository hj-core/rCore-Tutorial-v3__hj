use core::{arch::asm, cmp::min, slice};

use crate::{
    log,
    task::{
        APP_BASE_PTR_0, APP_MAX_NUMBER, APP_MAX_SIZE, KernelStack, TASK_CONTROL_BLOCK, UserStack,
        control::TaskState,
    },
    trap::{self, TrapContext},
    warn,
};

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

/// `get_total_apps` returns the total number of user apps, limited by
/// [APP_MAX_NUMBER]. To obtain the number of apps discovered, please use
/// [get_total_apps_found].
pub(crate) fn get_total_apps() -> usize {
    min(get_total_apps_found(), APP_MAX_NUMBER)
}

/// `get_total_apps_found` returns the total number of user apps found.
/// This value may be greater than [APP_MAX_NUMBER].
pub(super) fn get_total_apps_found() -> usize {
    unsafe { get_info_base_ptr().read() as usize }
}

/// `get_app_name` returns the name of the app, or an empty string if the
/// app index is invalid or the app name is invalid. Only ASCII characters
/// are supported.
pub(crate) fn get_app_name<'a>(app_index: usize) -> &'a str {
    if app_index >= get_total_apps() {
        return "";
    }
    let app_name_ptr = unsafe {
        get_info_base_ptr()
            .add(app_index * APP_META_SIZE + 1)
            .read() as *const u8
    };
    let app_name_len = get_app_data_start(app_index) - (app_name_ptr as usize);
    let slice = unsafe { slice::from_raw_parts(app_name_ptr, app_name_len) };
    core::str::from_utf8(slice).unwrap_or("")
}

/// `get_app_data_start` returns the starting address of the app data
/// in the data section.
pub(crate) fn get_app_data_start(app_index: usize) -> usize {
    if app_index >= get_total_apps() {
        return 0;
    }
    unsafe {
        get_info_base_ptr()
            .add(app_index * APP_META_SIZE + 2)
            .read() as usize
    }
}

/// `get_app_data_end` returns the end address (exclusive) of the app
/// data in the data section.
pub(crate) fn get_app_data_end(app_index: usize) -> usize {
    if app_index >= get_total_apps() {
        return 0;
    }
    unsafe {
        get_info_base_ptr()
            .add(app_index * APP_META_SIZE + 3)
            .read() as usize
    }
}

/// `get_app_base_ptr` returns a pointer to the agreed-upon memory address
/// where the app should be installed.
fn get_app_base_ptr(app_index: usize) -> *mut u8 {
    unsafe { APP_BASE_PTR_0.add(app_index * APP_MAX_SIZE) }
}

/// `install_all_apps` copies all user app data to the agreed-upon memory
/// addresses and returns the number of failed installations.
pub(super) fn install_all_apps() -> usize {
    let mut result = 0;
    for app_index in 0..get_total_apps() {
        if install_app_data(app_index) == 0 {
            warn!(
                "Failed to install user app: index={}, name={}",
                app_index,
                get_app_name(app_index)
            );
            result += 1;
        } else {
            push_init_trap_context(app_index);
            set_first_run_tcb(app_index);
        }
    }

    // Prevent CPU from using outdated instruction cache
    unsafe { asm!("fence.i") };

    result
}

/// `install_app` copies the app data to the agreed-upon memory address and
/// returns the number of bytes copied.
fn install_app_data(app_index: usize) -> usize {
    if app_index >= get_total_apps() {
        return 0;
    }

    let app_data_start = get_app_data_start(app_index);
    let app_data_end = get_app_data_end(app_index);
    let app_size = app_data_end - app_data_start;

    if app_size > APP_MAX_SIZE {
        return 0;
    }

    // Clear the reserved memory range
    let app_base_ptr = get_app_base_ptr(app_index);
    for i in 0..APP_MAX_SIZE {
        unsafe { app_base_ptr.add(i).write_volatile(0) };
    }

    // Copy the app data to the reserved memory range
    let app_data = unsafe { slice::from_raw_parts(app_data_start as *const u8, app_size) };
    let dst = unsafe { slice::from_raw_parts_mut(app_base_ptr, app_size) };
    dst.copy_from_slice(app_data);

    app_size
}

/// `push_init_trap_context` pushes an initial trap context onto the app's kernel stack,
/// to prepare for starting the app.
fn push_init_trap_context(app_index: usize) {
    let mut kernel_sp = KernelStack::get_upper_bound(app_index) as *mut TrapContext;
    assert!(
        kernel_sp.is_aligned(),
        "Actions required to align the kernel_sp with TrapContext"
    );

    let app_base_addr = get_app_base_ptr(app_index).addr();
    let init_context =
        TrapContext::new_app_context(app_base_addr, UserStack::get_upper_bound(app_index));
    unsafe {
        kernel_sp = kernel_sp.offset(-1);
        kernel_sp.write_volatile(init_context);
    };
}

/// `set_first_run_tcb` configures the [TaskControlBlock] of the app for its first
/// run.
///
/// [TaskControlBlock]: super::control::TaskControlBlock
fn set_first_run_tcb(app_index: usize) {
    let init_kernel_sp = KernelStack::get_upper_bound(app_index) - size_of::<TrapContext>();

    let mut tcb = TASK_CONTROL_BLOCK[app_index].lock();
    tcb.change_state(TaskState::Ready);

    let context = tcb.get_mut_context();
    context.set_ra(trap::__restore as usize);
    context.set_sp(init_kernel_sp);
}

/// `can_app_read_addr` returns whether the address is readable by the currently running
/// app. Clients should ensure that an app is indeed running; otherswis, the returned
/// result is invalid.
pub(crate) fn can_app_read_addr(app_index: usize, addr: usize) -> bool {
    let app_size = get_app_data_end(app_index) - get_app_data_start(app_index);
    if app_size == 0 {
        return false;
    }

    let app_base_addr = get_app_base_ptr(app_index).addr();
    let data_range = app_base_addr..(app_base_addr + app_size);
    if data_range.contains(&addr) {
        return true;
    }

    let stack_range = UserStack::get_lower_bound(app_index)..UserStack::get_upper_bound(app_index);
    if stack_range.contains(&addr) {
        return true;
    }

    false
}
