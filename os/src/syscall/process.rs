//! Process management syscalls

use crate::task::{exit_current_and_run_next, suspend_current_and_run_next, get_current_taskid};

use crate::timer::{get_time_ms, get_kcnt, get_ucnt};

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    let taskid = get_current_taskid();
    debug!("[kernel] Application{} exited with code {}", taskid, exit_code);
    debug!("[kernel] running time: {}ms(user), {}ms(kernel)", get_ucnt(taskid), get_kcnt(taskid));
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    //debug!("task{} called yield", get_current_taskid());
    suspend_current_and_run_next();
    0
}

/// get current time
pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}
