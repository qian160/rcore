use core::arch::asm;

const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_YIELD: usize = 124;
const SYS_GET_TIME: usize = 169;

const SYS_MMAP: usize = 222;
const SYS_MUNMAP: usize = 215;

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
    syscall(SYS_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(exit_code: i32) -> isize {
    syscall(SYS_EXIT, [exit_code as usize, 0, 0])
}

pub fn sys_yield() -> isize {
    syscall(SYS_YIELD, [0, 0, 0])
}

pub fn sys_get_time() -> isize {
    syscall(SYS_GET_TIME, [0, 0, 0])
}

pub fn sys_mmap(start_va: usize, len: usize, perm: usize) -> isize {
    syscall(SYS_MMAP, [start_va, len, perm])
}
pub fn sys_munmap(start_va: usize, len: usize) -> isize {
    syscall(SYS_MUNMAP, [start_va, len, 0])
}