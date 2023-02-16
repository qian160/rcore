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
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;

const SYSCALL_MMAP: usize = 222;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_LS: usize = 216;
const SYSCALL_SPAWN: usize = 400;
const SYSCALL_LINKAT: usize = 37;
const SYSCALL_UNLINKAT: usize = 35;
const SYSCALL_FSTAT: usize = 80;


mod fs;
mod process;

use fs::*;
use process::*;

use crate::{mm::{VirtAddr, MapPermission, VirtPageNum}, task::current_task};
static mut TIMER: usize = 0;
// count run time here
/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    let time_before = crate::timer::get_time_ms();
    unsafe{
        current_task().unwrap().inner_exclusive_access().increase_user_timer(time_before - TIMER);
        TIMER = time_before;
    }
    let ret = match syscall_id {
        SYSCALL_OPEN => sys_open(args[0] as *const u8, args[1] as u32),
        SYSCALL_CLOSE => sys_close(args[0]),
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_LS => sys_ls(),
        SYSCALL_SPAWN => sys_spawn(args[0] as *const u8),
        SYSCALL_LINKAT => sys_linkat(args[0] as *const u8, args[1] as *const u8),
        SYSCALL_UNLINKAT => sys_unlinkat(args[0] as *const u8),
        SYSCALL_FSTAT => 0,
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    };
    let time_after = crate::timer::get_time_ms();
    current_task().unwrap().inner_exclusive_access().increase_kernel_timer(time_after - time_before);
    ret
}

// none-standard syscall defined by myself

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
    let binding = current_task().unwrap();
    let current = &mut binding.inner_exclusive_access();

    let start_vpn = VirtPageNum::from(start).0;
    let end_vpn = VirtPageNum::from(start + len).0;
    for vpn in start_vpn..end_vpn{
        if !current.memory_set.page_table.translate(VirtPageNum(vpn)).is_none(){
            error!(" mmap failed. vpn: {:x} already mapped!", vpn);
            return -1;
        }
    }
    current.memory_set.insert_framed_area(start.into(),(start + len).into(), perm);
    0
//    current_task.memory_set.insert_framed_area(start.into(), (start+len).into(), perm);
//    len as isize
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
    let binding = current_task().unwrap();
    let current = &mut binding.inner_exclusive_access();
    let memory_set = &mut current.memory_set;
    let pgtbl = &mut memory_set.page_table;
    // check unmapped area
    let mut start_vpn = VirtPageNum::from(start).0;
    let end_vpn = VirtPageNum::from(start + len).0;
    for vpn in start_vpn..end_vpn{
        if pgtbl.translate(vpn.into()).is_none(){
            error!(" munmap failed. vpn: {:x} not mapped yet", vpn);
            return -1;
        }
    }
    trace!(" try to unmap vpn: {:x}, len = {:x}", start_vpn, len);
    for area in &mut memory_set.areas{
        //debug!(" [{:x}, {:x}]", area.vpn_range.get_start().0, area.vpn_range.get_end().0);
        if area.vpn_range.contain(VirtPageNum(start_vpn)) {
            area.unmap_one(pgtbl, VirtPageNum(start_vpn));
            trace!(" vpn {:x} unmapped!", start_vpn);
            start_vpn += 1;
        }
    }
    0
}
/// list all the apps
pub fn sys_ls() -> isize{
    crate::fs::list_apps();
    0
}