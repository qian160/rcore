use core::arch::asm;

const SYSCALL_WRITE: usize  = 64;
const SYSCALL_EXIT: usize   = 93;

// NONE-STANDARD
const SYSCALL_TRACE: usize  = 94;
const SYSCALL_TASKID: usize = 95;

fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        );
    }
    ret
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(exit_code: i32) -> isize {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0])
}

// these syscalls below are none-standard and can't be implemented by ecall
pub fn sys_trace() -> isize{
    syscall(SYSCALL_TRACE, [0; 3]);
    0
}

pub fn sys_taskid() -> isize {
    syscall(SYSCALL_TASKID, [0; 3])
}
