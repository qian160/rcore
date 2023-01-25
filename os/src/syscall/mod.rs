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

const SYSCALL_WRITE: usize   = 64;
const SYSCALL_EXIT: usize    = 93;
const SYSCALL_TRACE: usize   = 94; 
const SYSCALL_TASKID: usize  = 95; 

mod fs;
mod process;
pub mod util;

use fs::*;
use process::*;
use util::*;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_TRACE => unsafe  {sys_trace()},
        SYSCALL_TASKID => sys_taskid(),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
