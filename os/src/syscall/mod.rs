//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;

const SYSCALL_TRACE: usize = 94; 
const SYSCALL_TASKINFO: usize = 410; 

use crate::config::MAX_APP_NUM;
use crate::timer::{get_time_ms, get_kcnt, get_ucnt, APP_RUNTIME_CNT};
use crate::task::{get_current_taskid, TaskInfo, get_taskinfo};
use core::{arch::asm, ptr};
// use this to calculate u mode running time
// must be initialized to app's boot time(before 1st app's running)
/// last time when app entering the kernel
/// note: this time - last time = running time in U mode
pub static mut LAST_ENTERING_TIME: usize = 0;

mod fs;
mod process;

use fs::*;
use process::*;

/// handle syscall exception with `syscall_id` and other arguments
/// also count app's runtime
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    let time_at_start = get_time_ms();
    let taskid = get_current_taskid();
    unsafe {
        APP_RUNTIME_CNT[taskid].0 += time_at_start - LAST_ENTERING_TIME;
        LAST_ENTERING_TIME = time_at_start;
    }
    let ret = match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_TRACE => unsafe  {sys_trace()},
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(),
        SYSCALL_TASKINFO => sys_taskinfo(args[0], args[1] as *mut TaskInfo),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    };
    unsafe {
        let time_now = get_time_ms();
        APP_RUNTIME_CNT[taskid].1 += time_now - time_at_start;
    }
    ret
}

// none-standard syscall defined by myself

/*
risc-v stack frame:

-------------------- high (fp)
*   return address
*   prev fp
    saved registers
    local variables
    ...
-------------------- low (sp)
*/
/// print stack frame
pub unsafe fn sys_trace() -> isize {
    let mut fp: *const usize;
    asm!("mv {}, fp", out(reg) fp);

    println!("\t\t== Begin stack trace ==");
    while fp != ptr::null() {
        let saved_ra = *fp.sub(1);
        let saved_fp = *fp.sub(2);

        println!("ra = 0x{:016x}, prev fp = 0x{:016x}", saved_ra, saved_fp);

        fp = saved_fp as *const usize;
    }
    println!("\t\t== End stack trace ==");
    0
}
/// get the specified task's info
pub fn sys_taskinfo(id: usize, info: *mut TaskInfo) -> isize{
    if id < MAX_APP_NUM {
        unsafe {
            let temp = get_taskinfo(id);
            (*info).id = temp.id;
            (*info).status = temp.status;
            (*info).times = temp.times;
        }
        0
    }
    else {
        -1
    }
}