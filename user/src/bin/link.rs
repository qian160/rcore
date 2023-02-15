#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use user_lib::{close, open, read, OpenFlags, linkat};

#[no_mangle]
pub fn main() -> i32 {
    println!("test sys_linkat and sys_unlinkat...");
    println!("call linkat and try to use another name to access filea");
    let link_src = "114514\0";
    let link_target = "filea\0";
    let _ret = linkat(link_target.as_ptr(), link_src.as_ptr());
    let fd = open("114514\0", OpenFlags::RDONLY);
    if fd == -1 {
        panic!("Error occured when opening file");
    }
    println!("file {} opened successfully!", link_src);
    let fd = fd as usize;
    let mut buf = [0u8; 256];
    loop {
        let size = read(fd, &mut buf) as usize;
        if size == 0 {
            break;
        }
        println!("{}", core::str::from_utf8(&buf[..size]).unwrap());
    }
    println!("test linkat success!");
    close(fd);
    0
}