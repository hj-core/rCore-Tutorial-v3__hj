pub(crate) use super::change_current_task_state;
pub(crate) use super::is_current_task_running;

pub(crate) use super::loader::can_app_read_addr;
pub(crate) use super::loader::get_app_data_end;
pub(crate) use super::loader::get_app_data_start;
pub(crate) use super::loader::get_app_name;
pub(crate) use super::loader::get_total_apps;

pub(crate) use super::runner::get_current_app_index;
pub(crate) use super::runner::run_next_app;

pub(crate) use super::control::TaskState;
