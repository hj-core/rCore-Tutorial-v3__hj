use core::slice;
use core::str;

use crate::{debug, log};

/// The number of meta information items kept for each app
/// in the generated link_apps.S.
///
/// Currently, we keep the following for each app:
/// - name start va
/// - name end va (exclusive)
/// - elf start va
/// - elf end va (exclusive)
const APP_META_ITEMS: usize = 4;

/// Returns a pointer to the _num_apps, which is defined
/// in the generated link_app.S.
fn get_app_meta_base() -> *const u64 {
    unsafe extern "C" {
        unsafe fn _num_apps();
    }
    _num_apps as *const u64
}

/// Returns the total number of user apps.
pub(crate) fn get_total_apps() -> usize {
    unsafe { get_app_meta_base().read() as usize }
}

/// Returns the name of the app, or an empty string if
/// the app's index is invalid or if the app name contains
/// characters other than ASCII.
pub(super) fn get_app_name<'a>(app_index: usize) -> &'a str {
    if app_index >= get_total_apps() {
        return "";
    }
    let name_start = unsafe {
        get_app_meta_base()
            .add(app_index * APP_META_ITEMS + 1)
            .read()
    };
    let name_end = unsafe {
        get_app_meta_base()
            .add(app_index * APP_META_ITEMS + 2)
            .read()
    };
    let name_len = (name_end - name_start) as usize;
    let slice = unsafe { slice::from_raw_parts(name_start as *const u8, name_len) };
    str::from_utf8(slice).unwrap_or("")
}

/// Returns the app's ELF bytes.
pub(crate) fn get_app_elf<'a>(app_index: usize) -> &'a [u8] {
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
            .add(app_index * APP_META_ITEMS + 3)
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
            .add(app_index * APP_META_ITEMS + 4)
            .read() as usize
    }
}

pub(crate) fn log_app_elfs() {
    let total_apps = get_total_apps();

    for i in 0..total_apps {
        let name = get_app_name(i);
        let start = get_app_elf_start(i);
        let end = get_app_elf_end(i);
        let size = end - start;

        debug!(
            "app {} elf [{:#x}, {:#x}) size={}, name={}",
            i, start, end, size, name
        );
    }
}
