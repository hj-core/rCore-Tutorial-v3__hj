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

    pub(super) fn get_statistics(&self) -> &TaskStatistics {
        &self.statistics
    }

    /// `record_first_run_start` records the current mtime as the start
    /// time of the task's first run. It is a no-op if the task has been
    /// run previously.
    pub(super) fn record_first_run_start(&mut self) {
        let time = timer::read_time();
        if self.statistics.get_first_run_start_mtime() == 0 {
            self.statistics.set_first_run_start_mtime(time);
        }
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
pub(super) struct TaskStatistics {
    mtime_first_run_start: usize,
}

impl TaskStatistics {
    fn new_init() -> Self {
        Self {
            mtime_first_run_start: 0,
        }
    }

    fn set_first_run_start_mtime(&mut self, value: usize) {
        self.mtime_first_run_start = value;
    }

    fn get_first_run_start_mtime(&self) -> usize {
        self.mtime_first_run_start
    }
}
