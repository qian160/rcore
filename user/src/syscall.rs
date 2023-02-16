use core::arch::asm;

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

pub fn sys_open(path: &str, flags: u32) -> isize {
    syscall(SYSCALL_OPEN, [path.as_ptr() as usize, flags as usize, 0])
}

pub fn sys_close(fd: usize) -> isize {
    syscall(SYSCALL_CLOSE, [fd, 0, 0])
}

pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall(
        SYSCALL_READ,
        [fd, buffer.as_mut_ptr() as usize, buffer.len()],
    )
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0]);
    panic!("sys_exit never returns!");
}

pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

pub fn sys_get_time() -> isize {
    syscall(SYSCALL_GET_TIME, [0, 0, 0])
}

pub fn sys_getpid() -> isize {
    syscall(SYSCALL_GETPID, [0, 0, 0])
}

pub fn sys_fork() -> isize {
    syscall(SYSCALL_FORK, [0, 0, 0])
}

pub fn sys_exec(path: &str) -> isize {
    syscall(SYSCALL_EXEC, [path.as_ptr() as usize, 0, 0])
}

pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall(SYSCALL_WAITPID, [pid as usize, exit_code as usize, 0])
}

pub fn sys_ls() -> isize {
    syscall(SYSCALL_LS, [0; 3])
}

#[allow(unused)]
pub fn sys_mmap(start: usize, len: usize, perm: usize) -> isize {
    syscall(SYSCALL_MMAP, [start, len, perm])
}

#[allow(unused)]
pub fn sys_munmap(start: usize, len: usize) -> isize {
    syscall(SYSCALL_MUNMAP, [start, len, 0])
}

#[allow(unused)]
pub fn sys_spawn(file: *const u8) -> isize {
    syscall(SYSCALL_SPAWN, [file as usize, 0, 0])
}

#[allow(unused)]
pub fn sys_linkat(oldfile: *const u8, newfile: *const u8) -> isize {
    syscall(SYSCALL_LINKAT, [oldfile as usize, newfile as usize, 0])
}

#[allow(unused)]
pub fn sys_unlinkat(path: *const u8) -> isize {
    syscall(SYSCALL_UNLINKAT, [path as usize, 0, 0])
}

#[allow(unused)]
pub fn sys_fstat(fd: usize, st: *mut crate::Stat) -> isize {
    syscall(SYSCALL_FSTAT, [fd, st as *mut u8 as usize, 0])
}