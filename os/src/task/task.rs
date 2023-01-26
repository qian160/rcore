//! Types related to task management

use super::TaskContext;

#[derive(Copy, Clone)]
/// status(enum) and context(ra, sp, s)
/// be initialized in task/mod.rs 's lazy_static!
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}
