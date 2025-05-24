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
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TaskState {
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
}
