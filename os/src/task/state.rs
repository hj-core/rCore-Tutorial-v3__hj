use core::sync::atomic::{AtomicUsize, Ordering};

use crate::mm::prelude::VMSpace;
use crate::timer;

static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(1);

pub(super) const MAX_SYSCALLS_TRACKED: usize = 6;

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct TaskControlBlock {
    task_id: usize,
    vm_space: VMSpace,
    state: TaskState,
    context: TaskContext,
    statistics: TaskStatistics,
}

impl TaskControlBlock {
    pub(super) fn new_ready(
        vm_space: VMSpace,
        ra: usize,
        kernel_sp: usize,
        tp: usize,
        satp: usize,
    ) -> Self {
        Self {
            task_id: NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed),
            vm_space,
            state: TaskState::Ready,
            context: TaskContext::new_initial(ra, kernel_sp, tp, satp),
            statistics: TaskStatistics::new_zeros(),
        }
    }

    pub(super) fn get_task_id(&self) -> usize {
        self.task_id
    }

    pub(crate) fn get_vm_space_mut(&mut self) -> &mut VMSpace {
        &mut self.vm_space
    }

    pub(super) fn get_state(&self) -> TaskState {
        self.state
    }

    pub(super) fn set_state(&mut self, new_state: TaskState) {
        self.state = new_state;
    }

    pub(super) fn get_context(&self) -> &TaskContext {
        &self.context
    }

    pub(super) fn get_context_mut(&mut self) -> &mut TaskContext {
        &mut self.context
    }

    pub(super) fn get_statistics(&self) -> TaskStatistics {
        self.statistics
    }

    /// Records the current mtime as the task's last run start
    /// time and updates relevant statistics.
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

    /// Records the current mtime as the task's last run end
    /// time and updates relevant statistics.
    pub(super) fn record_run_end(&mut self) {
        let time = timer::read_time();
        self.statistics.set_last_run_end_mtime(time);

        let executed_time = time - self.statistics.get_last_run_start_mtime();
        self.statistics.increase_total_executed_mtime(executed_time);

        self.statistics.increase_switch_count();
    }

    /// Records a call to the given syscall_id for the task.
    /// This function has the same semantics as
    /// [TaskStatistics::increase_syscall_count].
    pub(super) fn record_syscall(&mut self, syscall_id: usize) -> bool {
        self.statistics.increase_syscall_count(syscall_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TaskState {
    Ready,
    Running,
    Killed,
    Exited,
}

#[derive(Debug)]
#[repr(C)]
pub(super) struct TaskContext {
    /// Callee-saved registers s0 through s11
    s: [usize; 12],
    ra: usize,
    sp: usize,
    tp: usize,
    satp: usize,
}

impl TaskContext {
    pub(super) fn new_initial(ra: usize, sp: usize, tp: usize, satp: usize) -> Self {
        Self {
            s: [0; 12],
            ra,
            sp,
            tp,
            satp,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TaskStatistics {
    mtime_first_run_start: usize,
    mtime_last_run_start: usize,
    mtime_last_run_end: usize,
    mtime_total_executed: usize,
    /// The total waiting time of a task, accumulating from
    /// its first run. A task is considered waiting if it is
    /// switched out before it is completed.
    mtime_total_waiting: usize,
    /// The number of times a task has been switched out,
    /// including the one when it is completed.
    switch_count: usize,
    /// The (syscall_id, called_times) statistics of a task.
    /// Can only track up to [MAX_SYSCALL_TRACKED] different
    /// syscalls.
    syscall_counts: [(usize, usize); MAX_SYSCALLS_TRACKED],
}

impl TaskStatistics {
    fn new_zeros() -> Self {
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

    /// Increases the count for the given syscall_id, which
    /// may fail since only the first [MAX_SYSCALLS_TRACKED]
    /// different syscalls are tracked. It returns a boolean
    /// indicating whether the call has been recorded.
    fn increase_syscall_count(&mut self, syscall_id: usize) -> bool {
        if let Some(i) = (0..MAX_SYSCALLS_TRACKED)
            .find(|&i| self.syscall_counts[i].0 == syscall_id || self.syscall_counts[i].0 == 0)
        {
            self.syscall_counts[i].0 = syscall_id;
            self.syscall_counts[i].1 += 1;
            true
        } else {
            false
        }
    }
}
