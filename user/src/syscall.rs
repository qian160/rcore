use core::arch::asm;

const SYS_READ: usize = 63;
const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_YIELD: usize = 124;
const SYS_GET_TIME: usize = 169;
const SYS_GETPID: usize = 172;
const SYS_FORK: usize = 220;
const SYS_EXEC: usize = 221;
const SYS_WAITPID: usize = 260;

const SYS_TRACE: usize = 94; 
const SYS_MMAP: usize = 222;
const SYS_MUNMAP: usize = 215;
const SYS_SPAWN: usize = 400;

const SYS_LS: usize = 216;


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

pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall(
        SYS_READ,
        [fd, buffer.as_mut_ptr() as usize, buffer.len()],
    )
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYS_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYS_EXIT, [exit_code as usize, 0, 0]);
    panic!("sys_exit never returns!");
}

pub fn sys_yield() -> isize {
    syscall(SYS_YIELD, [0, 0, 0])
}

pub fn sys_get_time() -> isize {
    syscall(SYS_GET_TIME, [0, 0, 0])
}

pub fn sys_getpid() -> isize {
    syscall(SYS_GETPID, [0, 0, 0])
}

pub fn sys_fork() -> isize {
    syscall(SYS_FORK, [0, 0, 0])
}

pub fn sys_exec(path: &str) -> isize {
    syscall(SYS_EXEC, [path.as_ptr() as usize, 0, 0])
}

pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall(SYS_WAITPID, [pid as usize, exit_code as usize, 0])
}

pub fn sys_ls() -> isize {
    syscall(SYS_LS, [0; 3])
}
#[allow(unused)]
pub fn sys_trace() -> isize {
    syscall(SYS_TRACE, [0; 3])
}
#[allow(unused)]
pub fn sys_mmap(start: usize, len: usize, perm: usize) -> isize {
    syscall(SYS_MMAP, [start, len, perm])
}
#[allow(unused)]
pub fn sys_munmap(start: usize, len: usize) -> isize {
    syscall(SYS_MUNMAP, [start, len, 0])
}

pub fn sys_spawn(file: *const u8) -> isize {
    syscall(SYS_SPAWN, [file as usize, 0, 0])
}