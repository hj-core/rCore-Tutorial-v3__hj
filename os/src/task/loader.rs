use core::str;
use core::{cmp::min, slice};

use crate::{debug, log};

/// The agreed-upon address where the first user app should be
/// installed.
const APP_ENTRY_PTR_0: *mut u8 = 0x8080_0000 as *mut u8;
const APP_MAX_SIZE: usize = 0x2_0000;
const APP_MAX_NUMBER: usize = super::TASK_MAX_NUMBER;

/// The number of meta information items kept for each app in the
/// generated link_apps.S.
///
/// Currently, we keep the following for each app:
/// - name start
/// - name end
/// - elf start
/// - elf end
const APP_META_SIZE: usize = 4;

/// Returns a pointer to the _num_apps, which is defined in the
/// generated link_app.S.
fn get_app_meta_base() -> *const u64 {
    unsafe extern "C" {
        unsafe fn _num_apps();
    }
    _num_apps as *const u64
}

/// Returns the total number of user apps, limited by
/// [APP_MAX_NUMBER].
///
/// To obtain the number of apps discovered, please use
/// [get_total_apps_found].
pub(crate) fn get_total_apps() -> usize {
    min(get_total_apps_found(), APP_MAX_NUMBER)
}

/// Returns the total number of user apps found. This value may
/// be greater than [APP_MAX_NUMBER].
fn get_total_apps_found() -> usize {
    unsafe { get_app_meta_base().read() as usize }
}

/// Returns the name of the app, or an empty string if the app's
/// index is invalid or if the app name contains characters other
/// than ASCII.
pub(super) fn get_app_name<'a>(app_index: usize) -> &'a str {
    if app_index >= get_total_apps() {
        return "";
    }
    let name_start = unsafe {
        get_app_meta_base()
            .add(app_index * APP_META_SIZE + 1)
            .read()
    };
    let name_end = unsafe {
        get_app_meta_base()
            .add(app_index * APP_META_SIZE + 2)
            .read()
    };
    let name_len = (name_end - name_start) as usize;
    let slice = unsafe { slice::from_raw_parts(name_start as *const u8, name_len) };
    str::from_utf8(slice).unwrap_or("")
}

/// Returns the bytes of the app's ELF.
pub(crate) fn get_app_elf_bytes<'a>(app_index: usize) -> &'a [u8] {
    let elf_start = get_app_elf_start(app_index);
    let elf_end = get_app_elf_end(app_index);
    let elf_size = elf_end - elf_start;
    unsafe { slice::from_raw_parts(elf_start as *const u8, elf_size) }
}

/// Returns the starting address of the app's ELF in memory.
fn get_app_elf_start(app_index: usize) -> usize {
    if app_index >= get_total_apps() {
        return 0;
    }
    unsafe {
        get_app_meta_base()
            .add(app_index * APP_META_SIZE + 3)
            .read() as usize
    }
}

/// Returns the end address (exclusive) of the app's ELF in memory.
fn get_app_elf_end(app_index: usize) -> usize {
    if app_index >= get_total_apps() {
        return 0;
    }
    unsafe {
        get_app_meta_base()
            .add(app_index * APP_META_SIZE + 4)
            .read() as usize
    }
}

/// Returns a pointer to the agreed-upon memory address where
/// the app should be installed.
///
/// # Warning
/// * To be deprecated when virtual address for user space is
/// implemented.
pub(crate) fn get_app_entry_ptr(app_index: usize) -> *mut u8 {
    unsafe { APP_ENTRY_PTR_0.add(app_index * APP_MAX_SIZE) }
}

/// Returns whether the address is within the range of installed
/// app data.
///
/// # Warning
/// * To be deprecated when virtual address for user space is
/// implemented.
pub(super) fn is_app_installed_data(app_index: usize, addr: usize) -> bool {
    let app_size = get_app_elf_end(app_index) - get_app_elf_start(app_index);
    if app_size == 0 {
        return false;
    }

    let app_entry_addr = get_app_entry_ptr(app_index).addr();
    let installed_range = app_entry_addr..(app_entry_addr + app_size);
    installed_range.contains(&addr)
}

pub(crate) fn log_app_elfs_layout() {
    let total_apps = get_total_apps();

    for i in 0..total_apps {
        let elf_start = get_app_elf_start(i);
        let elf_end = get_app_elf_end(i);
        let elf_size = elf_end - elf_start;
        let app_name = get_app_name(i);

        debug!(
            "app_{} [{:#x}, {:#x}) size={}, name={}",
            i, elf_start, elf_end, elf_size, app_name
        );
    }
}
