//! none-standard syscall defined by myself
use core::{arch::asm, ptr};
use crate::{task::{TaskInfo, get_current_taskid, get_taskinfo}, config::MAX_APP_NUM};
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