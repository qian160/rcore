//! File and filesystem-related syscalls
use easy_fs::Inode;

use crate::fs::{open_file, OpenFlags, ROOT_INODE};
use crate::lang_items::trace;
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}
/// search the root inode
pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        trace!(" open fd = [{}], name = {}", fd, path);
        inner.fd_table[fd] = Some(inode);
        inner.fd_name_map.insert(fd as i32, path);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    inner.fd_name_map.remove(&(fd as i32));
    trace!(" close fd = [{}]", fd);
    0
}
/// link the target file to src. steps:
/// 1. create
#[allow(unused)]
pub fn sys_linkat(src: *const u8, target: *const u8) -> isize {
    let token = current_user_token();
    let new_name = translated_str(token, target);
    let old_name = translated_str(token, src);
    if old_name == new_name {
        error!("can not link a file to itself!");
        return -1;
    }
    let old_inode = ROOT_INODE.find(&old_name).unwrap();
    let mut new_inode = ROOT_INODE.create(&new_name).unwrap();
    new_inode.linkat(&old_inode);
    0
}

#[allow(unused)]
/// unlink a file from filesystem
pub fn sys_unlinkat(path: *const u8) -> isize {
    let token = current_user_token();
    let name = translated_str(token, path);
    if let Some(inode) = ROOT_INODE.find(&name).as_mut() {
        ROOT_INODE.unlink(&name);
        let mut buffer = [0; 512];
        assert_eq!(inode.read_at(0, &mut buffer), 0,);
        trace!(" unlink {}", name);
        return 0;
    }
    error!("unlink failed. file '{}' doesn't exist!", name);
    -1
}

#[repr(C)]
#[derive(Debug)]
pub struct Stat {
    /// 文件所在磁盘驱动器号，该实验中写死为 0 即可
    pub dev: u64,
    /// inode 文件所在 inode 编号
    pub ino: u64,
    /// 文件类型
    pub mode: StatMode,
    /// 硬链接数量，初始为1
    pub nlink: u32,
    /// 无需考虑，为了兼容性设计
    pad: [u64; 7],
}

bitflags! {
    /// StatMode 定义：
    pub struct StatMode: u32 {
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;
        /// ordinary regular file
        const FILE  = 0o100000;
    }
}
#[allow(unused)]
pub fn sys_fstat(fd: i32, st: *mut Stat) -> isize {
    // need to build a coeection between fd and inode
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let pgtbl = &inner.memory_set.page_table;
    let addr = pgtbl.translate_va((st as *mut u8 as usize).into()).unwrap().0;
    let addr = addr as *mut u8 as *mut Stat;
    if inner.fd_table[fd as usize].is_some() {
        let name = inner.fd_name_map.get(&fd).unwrap();
        let inode = ROOT_INODE.find(&name).unwrap();
        unsafe {
            *addr = Stat{
                dev: 0,
                ino: inode.size() as u64,
                mode:
                    if inode.is_file() {
                        StatMode::FILE
                    }
                    else {
                        StatMode::DIR
                    },
                nlink: 0,
                pad: [0; 7],
            };
        }
        trace!(" {:?}", st);
        return 0;
    }
    error!(" fd: [{}] not found!", fd);
    -1
}