//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::mm::vmprint;
use crate::timer::{get_time_ms, get_ucnt, get_kcnt};
use crate::loader::{get_app_data, get_num_app};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use crate::mm::PageTable;
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    pub inner: UPSafeCell<TaskManagerInner>,
}

/// The task manager inner in 'UPSafeCell'
pub struct TaskManagerInner {
    /// task list
    pub tasks: Vec<TaskControlBlock>,
    /// id of current `Running` task
    current_task: usize,
}

//pub fn get_tcb_vec() -> &mut

lazy_static! {
    /// a `TaskManager` global instance through lazy_static!
    /// read data into task from elf file
    pub static ref TASK_MANAGER: TaskManager = {
        info!(" init TASK_MANAGER");
        let num_app = get_num_app();
        info!(" num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            // 
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

/// get current taskid. mostly used one
pub fn get_current_taskid() -> usize {
    TASK_MANAGER.inner.exclusive_access().current_task
}
/// get the specified task's info
pub fn get_taskinfo(id: usize) -> TaskInfo {
    TaskInfo {
        id,
        status: TASK_MANAGER.inner.exclusive_access().tasks[id].task_status,
        times: (get_ucnt(id), get_kcnt(id)),
    }
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct TaskInfo {
    pub id: usize,
    pub status: TaskStatus,
    /// 0 for kernel, 1 for user
    pub times: (usize, usize)
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch4, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let first_task = &mut inner.tasks[0];
        first_task.task_status = TaskStatus::Running;
        let first_task_cx_ptr = &first_task.task_cx as *const TaskContext;
        let mut _unused = TaskContext::zero_init();
        unsafe{
            crate::syscall::LAST_ENTERING_TIME = get_time_ms();
        }
        // before this, we should drop local variables that must be dropped manually
        drop(inner);
        unsafe {
            __switch(&mut _unused as *mut _, first_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            //debug!("next task is {}", next);
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            statistic();
            use crate::board::QEMUExit;
            crate::board::QEMU_EXIT_HANDLE.exit_success();
        }
    }
}

fn statistic() {
    let n = get_num_app();
    let total_cnt_k: usize= (0..n).map(|i| get_kcnt(i)).sum();
    let total_cnt_u: usize= (0..n).map(|i| get_ucnt(i)).sum();
    println!("");
    debug!("All applications completed!");
    (0..n).for_each(|id| {
        trace!(" {:?}", get_taskinfo(id));
    });
    debug!("total running time: {}ms(user), {}ms(kernel)", total_cnt_u, total_cnt_k);
}

/// Run the first task in task list.
pub fn run_first_task() {
    // try to print first app's pagetable
    let inner = TASK_MANAGER.inner.exclusive_access();
    let token = inner.tasks[0].get_user_token();
    let pgtbl = PageTable::from_token(token);
    info!(" first task's pagetable");
    vmprint(&pgtbl);
    // don't forget to drop inner
    drop(inner);
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}