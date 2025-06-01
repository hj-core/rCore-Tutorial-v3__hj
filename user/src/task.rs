const MAX_SYSCALLS_TRACKED: usize = 6;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct TaskInfo {
    pub task_id: usize,
    pub state: TaskState,
    pub stastics: TaskStatistics,
}

impl TaskInfo {
    pub fn new_placeholder() -> Self {
        Self {
            task_id: 0,
            state: TaskState::Unused,
            stastics: TaskStatistics::new_init(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskState {
    Unused,
    Ready,
    Running,
    Killed,
    Exited,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct TaskStatistics {
    pub mtime_first_run_start: usize,
    pub mtime_last_run_start: usize,
    pub mtime_last_run_end: usize,
    pub mtime_total_executed: usize,
    /// The total waiting time of a task, accumulating from its first run.
    /// A task is considered waiting if it is switched out before it is
    /// completed.
    pub mtime_total_waiting: usize,
    /// The number of times a task has been switched out, including the one
    /// when it is completed.
    pub switch_count: usize,
    /// The (syscall_id, called_times) statistics of a task. Can only track
    /// up to [MAX_SYSCALL_TRACKED] different syscalls.
    pub syscall_counts: [(usize, usize); MAX_SYSCALLS_TRACKED],
}

impl TaskStatistics {
    pub fn new_init() -> Self {
        Self {
            mtime_first_run_start: 0,
            mtime_last_run_start: 0,
            mtime_last_run_end: 0,
            mtime_total_executed: 0,
            mtime_total_waiting: 0,
            switch_count: 0,
            syscall_counts: [(0, 0); MAX_SYSCALLS_TRACKED],
        }
    }
}
