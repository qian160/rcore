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
const SYS_FORK: usize = 114;


use crate::{loader::get_num_app, mm::VirtPageNum};
#[allow(unused)]
use crate::mm::{MapPermission, frame_alloc, PTEFlags, VirtAddr, PageTable};
use crate::timer::{get_time_ms, APP_RUNTIME_CNT};
#[allow(unused)]
use crate::task::{get_current_taskid, TaskInfo, get_taskinfo, current_user_token, TASK_MANAGER};
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
    assert!(prot > 0 && prot <= 7);
    assert!(VirtAddr::from(start).aligned());
    assert!(len > 0);
    let mut perm = MapPermission::U;
    if (prot & 1) == 1 {
        perm |= MapPermission::R;
    }
    if (prot & 2) == 2 {
        perm |= MapPermission::W;
    }
    if (prot & 4) == 4 {
        perm |= MapPermission::X;
    }
    let current = get_current_taskid();
    let current_task = &mut TASK_MANAGER.inner.exclusive_access().tasks[current];
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);

    let start_vpn = VirtPageNum::from(start_va).0;
    let end_vpn = VirtPageNum::from(end_va).0;
    for vpn in start_vpn..end_vpn{
        if !current_task.memory_set.page_table.translate(vpn.into()).is_none(){
            error!(" mmap failed");
            return -1;
        }
    }
    current_task.memory_set.insert_framed_area(start.into(), (start+len).into(), perm);
    len as isize
    /* 
    let flags = PTEFlags::from_bits(perm.bits()).unwrap();
    let pgtbl = &mut current_task.memory_set.page_table;
    let mut start_vpn = VirtAddr::from(start).floor();
    let end_vpn = VirtAddr::from(start + len).floor();
    while start_vpn <= end_vpn {
        assert!(pgtbl.translate(start_vpn).is_none());
        let frame = frame_alloc().unwrap();
        debug!(" ppn = {:x}", frame.ppn.0);
        pgtbl.map(start_vpn, frame.ppn, flags);
        start_vpn.0 += 1;
    }
    */
}
/// 取消到 [start, start + len) 虚存的映射
pub fn sys_munmap(start: usize, len: usize) -> isize {
    // not implemented
    let current = get_current_taskid();
    let current_task = &mut TASK_MANAGER.inner.exclusive_access().tasks[current];
    // check unmapped area

    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);

    let start_vpn = VirtPageNum::from(start_va).0;
    let end_vpn = VirtPageNum::from(end_va).0;

    trace!("start: {:x}", start);
    trace!("start: {:x}", VirtAddr::from(start).0);
    trace!("start: {:x}", VirtPageNum::from(start).0);
    trace!("start: {:x}", VirtPageNum::from(VirtAddr::from(start)).0);
    for vpn in start_vpn..end_vpn{
        if !current_task.memory_set.page_table.translate(vpn.into()).is_none(){
            error!(" munmap failed");
            return -1;
        }
    }  
    for area in &mut current_task.memory_set.areas{
        debug!(" [{:x}, {:x}]", area.vpn_range.get_start().0, area.vpn_range.get_end().0);
    }
    0
}