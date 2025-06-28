use core::{arch::asm, cmp::min, slice};

use crate::mm::prelude as mm_p;
use crate::{debug, log, warn};

/// The agreed-upon address where the first user app should be installed.
const APP_ENTRY_PTR_0: *mut u8 = 0x8080_0000 as *mut u8;
const APP_MAX_SIZE: usize = 0x2_0000;
const APP_MAX_NUMBER: usize = super::TASK_MAX_NUMBER;

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
fn get_total_apps_found() -> usize {
    unsafe { get_info_base_ptr().read() as usize }
}

/// `get_app_name` returns the name of the app, or an empty string if the
/// app index is invalid or the app name is invalid. Only ASCII characters
/// are supported.
pub(super) fn get_app_name<'a>(app_index: usize) -> &'a str {
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
fn get_app_data_start(app_index: usize) -> usize {
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
fn get_app_data_end(app_index: usize) -> usize {
    if app_index >= get_total_apps() {
        return 0;
    }
    unsafe {
        get_info_base_ptr()
            .add(app_index * APP_META_SIZE + 3)
            .read() as usize
    }
}

/// `get_app_entry_ptr` returns a pointer to the agreed-upon memory address
/// where the app should be installed.
pub(super) fn get_app_entry_ptr(app_index: usize) -> *mut u8 {
    unsafe { APP_ENTRY_PTR_0.add(app_index * APP_MAX_SIZE) }
}

/// `install_all_apps` copies the user apps (up to [APP_MAX_NUMBER]) to the
/// designated memory addresses and returns the number of failed installations.
pub(super) fn install_all_apps() -> usize {
    if APP_ENTRY_PTR_0.addr() < mm_p::get_kernel_end() {
        panic!("Kernel data extruded into the app-reserved addresses.");
    }

    if APP_MAX_NUMBER < get_total_apps_found() {
        warn!(
            "{} user apps found. Supports up to {}; the rest are ignored.",
            get_total_apps_found(),
            APP_MAX_NUMBER,
        );
    }

    let mut result = 0;
    for app_index in 0..get_total_apps() {
        if install_app_data(app_index) == 0 {
            warn!(
                "Failed to install user app: index={}, name={}",
                app_index,
                get_app_name(app_index)
            );
            result += 1;
        }
    }

    // Prevent CPU from using outdated instruction cache
    unsafe { asm!("fence.i") };

    result
}

/// `install_app_data` copies the app data to the designated memory addresses and
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
    let app_entry_ptr = get_app_entry_ptr(app_index);
    for i in 0..APP_MAX_SIZE {
        unsafe { app_entry_ptr.add(i).write_volatile(0) };
    }

    // Copy the app data to the reserved memory range
    let app_data = unsafe { slice::from_raw_parts(app_data_start as *const u8, app_size) };
    let dst = unsafe { slice::from_raw_parts_mut(app_entry_ptr, app_size) };
    dst.copy_from_slice(app_data);

    app_size
}

/// `is_app_installed_data` returns whether the address is within the range of
/// installed app data.
pub(super) fn is_app_installed_data(app_index: usize, addr: usize) -> bool {
    let app_size = get_app_data_end(app_index) - get_app_data_start(app_index);
    if app_size == 0 {
        return false;
    }

    let app_entry_addr = get_app_entry_ptr(app_index).addr();
    let installed_range = app_entry_addr..(app_entry_addr + app_size);
    installed_range.contains(&addr)
}

pub(crate) fn log_apps_layout() {
    let total_apps = get_total_apps();

    for i in 0..total_apps {
        let app_start = get_app_data_start(i);
        let app_end = get_app_data_end(i);
        let app_size = app_end - app_start;
        let app_name = get_app_name(i);

        debug!(
            "app_{} [{:#x}, {:#x}) size={}, name={}",
            i, app_start, app_end, app_size, app_name
        );
    }
}
