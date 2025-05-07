use core::slice;

pub struct AppManager;

impl AppManager {
    /// The agreed-upon address where the running app should be installed.
    const APP_MEM_ADDR: *mut u8 = 0x8040_0000 as *mut u8;
    const APP_MAX_SIZE: usize = 0x2_0000;

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

    /// `get_app_data_start` returns the starting address of the app data
    /// in the data section.
    pub fn get_app_data_start(app_index: usize) -> usize {
        if app_index >= Self::get_total_apps() {
            return 0;
        }
        unsafe { Self::get_info_base_ptr().add(app_index + 1).read() as usize }
    }

    /// `get_app_data_end` returns the end address (exclusive) of the app
    /// data in the data section.
    pub fn get_app_data_end(app_index: usize) -> usize {
        if app_index >= Self::get_total_apps() {
            return 0;
        }
        unsafe { Self::get_info_base_ptr().add(app_index + 2).read() as usize }
    }

    /// `install_app` copies the app data to the agreed-upon [Self::APP_MEM_ADDR],
    /// and returns the number of bytes copied.
    pub fn install_app(app_index: usize) -> usize {
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

        app_size
    }
}
