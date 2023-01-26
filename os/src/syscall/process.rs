//! Process management syscalls
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next, get_current_taskid};
use crate::timer::get_time_ms;
use crate::syscall::{get_kcnt, get_ucnt};

/// task exits and submit an exit code. show the task's info before it exits
pub fn sys_exit(exit_code: i32) -> ! {
    let taskid = get_current_taskid();
    debug!("[kernel] Application{} exited with code {}", taskid, exit_code);
    debug!("running time: {}ms(kernel), {}ms(user)", get_kcnt(taskid), get_ucnt(taskid));
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    info!("task{} called yield", get_current_taskid());
    suspend_current_and_run_next();
    0
}

/// get time in milliseconds
pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}
