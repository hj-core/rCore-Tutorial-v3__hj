#[derive(Debug)]
pub(super) struct TaskControlBlock {
    state: TaskState,
    context: TaskContext,
}

impl TaskControlBlock {
    pub(super) fn new_placeholder() -> Self {
        Self {
            state: TaskState::Unused,
            context: TaskContext::new_placeholder(),
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum TaskState {
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
