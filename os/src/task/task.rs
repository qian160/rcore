//!Implementation of [`TaskControlBlock`]
use super::TaskContext;
use super::{pid_alloc, KernelStack, PidHandle};
use crate::config::TRAP_CONTEXT;
use crate::mm::{MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::sync::UPSafeCell;
use crate::trap::{trap_handler, TrapContext};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::cell::RefMut;

/// 任务控制块中包含两部分：
/// 1. 在初始化之后就不再变化的元数据：直接放在任务控制块中。(pid, kstack)
/// 2. 在运行过程中可能发生变化的元数据：则放在 TaskControlBlockInner 中.
/// 将它再包裹上一层 UPSafeCell<T> 放在任务控制块中。这是因为在我们的设计中
/// 外层只能获取任务控制块的不可变引用，若想修改里面的部分内容的话这需要 UPSafeCell<T> 所提供的内部可变性。
pub struct TaskControlBlock {
    // immutable
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    // mutable
    inner: UPSafeCell<TaskControlBlockInner>,
}

/// note: the `children` and `parent` members just keep a pointer.
/// the true data is located in heap.
pub struct TaskControlBlockInner {
    // trap_cx_ppn is a little redundant. we could also find that
    // by looking up pagetable, like: let trap_cx_ppn = pagetable.translate(TRAP_CONTEXT.into()).ppn
    // here is just a space vs time tradeoff
    pub trap_cx_ppn: PhysPageNum,
    /// 应用数据仅有可能出现在应用地址空间低于 base_size 字节的区域中. `init value = user_sp`
    pub base_size: usize,
    pub task_cx: TaskContext,
    pub task_status: TaskStatus,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,
    /// 当进程调用 exit 系统调用主动退出或者执行出错由内核终止的时候，
    /// 它的退出码 exit_code 会被内核保存在它的任务控制块中，
    /// 并等待它的父进程通过 waitpid 回收它的资源的同时也收集它的 PID 以及退出码
    pub exit_code: i32,
    pub runtime_in_user: usize,
    pub runtime_in_kernel: usize,
}

impl TaskControlBlockInner {
    /// return the trapframe's ppn
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }
    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
    pub fn increase_user_timer(&mut self, ms: usize){
        self.runtime_in_user += ms;
    }
    pub fn increase_kernel_timer(&mut self, ms: usize){
        self.runtime_in_kernel += ms;
    }
}

impl TaskControlBlock {
    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }
    pub fn new(elf_data: &[u8]) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        // push a task context which goes to trap_return to the top of kernel stack
        // the task will go to execuating in `__restore` with its 
        // carefully prepared init context
        let task_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    runtime_in_user: 0,
                    runtime_in_kernel: 0,
                })
            },
        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }
    /// construct a new tcb from elf_data and use that to rewrite current's
    pub fn exec(&self, elf_data: &[u8]) {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // **** access inner exclusively
        let mut inner = self.inner_exclusive_access();
        // substitute memory_set
        inner.memory_set = memory_set;
        // update trap_cx ppn
        inner.trap_cx_ppn = trap_cx_ppn;
        // initialize base_size
        inner.base_size = user_sp;
        // initialize trap_cx
        let trap_cx = inner.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
        // **** release inner automatically
    }
    /// makes a copy and returns the child
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        // ---- access parent PCB exclusively
        let mut parent_inner = self.inner_exclusive_access();
        // copy user space(include trap context)
        let memory_set = MemorySet::from_existed_user(&parent_inner.memory_set);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        // alloc a tcb for child
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: parent_inner.base_size,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    runtime_in_user: 0,
                    runtime_in_kernel: 0,
                })
            },
        });
        // add child
        parent_inner.children.push(task_control_block.clone());
        // modify kernel_sp in trap_cx
        // **** access children PCB exclusively
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;
        // return the child
        task_control_block
        // ---- release parent PCB automatically
        // **** release children PCB automatically
    }
    /// create a child to execuate the target process
    pub fn spawn(self: &Arc<TaskControlBlock>, elf_data: &[u8]) -> Arc<TaskControlBlock> {
        // copy user space(include trap context)
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let mut parent_inner = self.inner_exclusive_access();
        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        // push a goto_trap_return task_cx on the top of kernel stack
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe{
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    runtime_in_kernel: 0,
                    runtime_in_user: 0,
                    task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            })
            },
        });
        // add child
        parent_inner.children.push(task_control_block.clone());
        // modify kernel_sp in trap_cx
        // **** acquire child PCB lock
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
        // **** release child PCB lock
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        // return
        task_control_block
        // ---- release parent PCB lock
}

    pub fn getpid(&self) -> usize {
        self.pid.0
    }
    pub fn show_timer_before_exit(&self){
        let utimer = self.inner_exclusive_access().runtime_in_user;
        let ktimer = self.inner_exclusive_access().runtime_in_kernel;
        debug!(" pid = {} exited. runtime: {}ms(user) {}ms(kernel)",
            self.pid.0, utimer, ktimer
        );
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

/// similar to tcb
#[derive(Debug)]
pub struct TaskInfo {
    ///
    pub root_pagetable: usize,
    ///
    pub trap_cx_ppn: PhysPageNum,
    ///
    pub base_size: usize,
    ///
    pub runtime_in_user: usize,
    ///
    pub runtime_in_kernel: usize,
}