#[derive(Debug)]
struct TaskControlBlock {
    state: TaskState,
    context: TaskContext,
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
