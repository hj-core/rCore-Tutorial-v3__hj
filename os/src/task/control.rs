use crate::timer;

#[derive(Debug)]
pub(super) struct TaskControlBlock {
    state: TaskState,
    context: TaskContext,
    statistics: TaskStatistics,
}

impl TaskControlBlock {
    pub(super) fn new_placeholder() -> Self {
        Self {
            state: TaskState::Unused,
            context: TaskContext::new_placeholder(),
            statistics: TaskStatistics::new_init(),
        }
    }

    pub(super) fn get_state(&self) -> TaskState {
        self.state
    }

    pub(super) fn change_state(&mut self, new_state: TaskState) {
        self.state = new_state;
    }

    pub(super) fn get_context(&self) -> &TaskContext {
        &self.context
    }

    pub(super) fn get_mut_context(&mut self) -> &mut TaskContext {
        &mut self.context
    }

    /// `record_run_start` records the current mtime as the task's last
    /// run start time. It additionally sets the task's first run start
    /// time if it is the first run of the task.
    pub(super) fn record_run_start(&mut self) {
        let time = timer::read_time();

        if self.statistics.get_first_run_start_mtime() == 0 {
            self.statistics.set_first_run_start_mtime(time);
        } else {
            let waiting_time = time - self.statistics.get_last_run_end_time();
            self.statistics.increase_total_waiting_mtime(waiting_time);
        }

        self.statistics.set_last_run_start_mtime(time);
    }

    /// `record_run_end` records the current mtime as the task's last
    /// run end time, and updates the total executed time and the switch
    /// count.
    pub(super) fn record_run_end(&mut self) {
        let time = timer::read_time();
        self.statistics.set_last_run_end_mtime(time);

        let executed_time = time - self.statistics.get_last_run_start_mtime();
        self.statistics.increase_total_executed_mtime(executed_time);

        self.statistics.increase_switch_count();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TaskState {
    Unused,
    Ready,
    Running,
    Killed,
    Exited,
}

#[derive(Debug)]
#[repr(C)]
pub(super) struct TaskContext {
    ra: usize,
    sp: usize,
    /// Callee-saved registers s0 through s11
    s: [usize; 12],
}

impl TaskContext {
    fn new_placeholder() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    pub(super) fn set_ra(&mut self, value: usize) {
        self.ra = value;
    }

    pub(super) fn set_sp(&mut self, value: usize) {
        self.sp = value;
    }
}

#[derive(Debug)]
struct TaskStatistics {
    mtime_first_run_start: usize,
    mtime_last_run_start: usize,
    mtime_last_run_end: usize,
    mtime_total_executed: usize,
    /// The total waiting time of a task, accumulating from its first run.
    /// A task is considered waiting if it is switched out before it is
    /// completed.
    mtime_total_waiting: usize,
    /// The number of times a task has been switched out, including the one
    /// when it is completed.
    switch_count: usize,
}

impl TaskStatistics {
    fn new_init() -> Self {
        Self {
            mtime_first_run_start: 0,
            mtime_last_run_start: 0,
            mtime_last_run_end: 0,
            mtime_total_executed: 0,
            mtime_total_waiting: 0,
            switch_count: 0,
        }
    }

    fn set_first_run_start_mtime(&mut self, value: usize) {
        self.mtime_first_run_start = value;
    }

    fn get_first_run_start_mtime(&self) -> usize {
        self.mtime_first_run_start
    }

    fn set_last_run_start_mtime(&mut self, value: usize) {
        self.mtime_last_run_start = value;
    }

    fn get_last_run_start_mtime(&self) -> usize {
        self.mtime_last_run_start
    }

    fn set_last_run_end_mtime(&mut self, value: usize) {
        self.mtime_last_run_end = value;
    }

    fn get_last_run_end_time(&self) -> usize {
        self.mtime_last_run_end
    }

    fn increase_total_executed_mtime(&mut self, value: usize) {
        self.mtime_total_executed += value;
    }

    fn increase_total_waiting_mtime(&mut self, value: usize) {
        self.mtime_total_waiting += value;
    }

    fn increase_switch_count(&mut self) {
        self.switch_count += 1;
    }
}
