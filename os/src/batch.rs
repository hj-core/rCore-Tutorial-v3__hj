pub struct AppManager;

impl AppManager {
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
}
