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

const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_YIELD: usize = 124;
const SYS_GET_TIME: usize = 169;

const SYS_TRACE: usize = 94; 
const SYS_TASKINFO: usize = 410;
const SYS_MMAP: usize = 222;
const SYS_MUNMAP: usize = 215;
const  SYS_FORK: usize = 114;

use crate::loader::get_num_app;
use crate::mm::{MapPermission, PageTable};
use crate::timer::{get_time_ms, APP_RUNTIME_CNT};
use crate::task::{get_current_taskid, TaskInfo, get_taskinfo, current_user_token};
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
        SYS_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYS_EXIT => sys_exit(args[0] as i32),
        SYS_TRACE => unsafe  {sys_trace()},
        SYS_YIELD => sys_yield(),
        SYS_GET_TIME => sys_get_time(),
        SYS_TASKINFO => sys_taskinfo(args[0], args[1] as *mut TaskInfo),
        SYS_MMAP => sys_mmap(args[0], args[1], args[2]),
        SYS_MUNMAP => sys_munmap(args[0], args[1]),
        SYS_FORK => sys_fork(),
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
/// get the specified task's info. need to be improved...
pub fn sys_taskinfo(id: usize, info: *mut TaskInfo) -> isize{
    if id < get_num_app() {
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

/// 申请长度为 len 字节的物理内存，将其映射到 start 开始的虚存，内存页属性为 prot
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    assert!(prot <= 7);
    let mut perm = MapPermission::empty();
    if (prot & 1) == 1 {
        perm |= MapPermission::R;
    }
    if (prot & 2) == 2 {
        perm |= MapPermission::W;
    }
    if (prot & 4) == 4 {
        perm |= MapPermission::X;
    }
    let _pgtbl = PageTable::from_token(current_user_token());
    (start + len) as isize
}
/// 取消到 [start, start + len) 虚存的映射
pub fn sys_munmap(start: usize, len: usize) -> isize {
    (start + len) as isize
}