#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use user_lib::{close, open, read, OpenFlags, linkat, unlinkat};

const link_src: &str = "114514\0";
const link_target: &str = "filea\0";

fn cat() {
    let fd = open("114514\0", OpenFlags::RDONLY);
    if fd == -1 {
        panic!("Error occured when opening file");
    }
    // bug: the contents inside are gone, but file can still be opened
    let fd = fd as usize;
    println!("file {} opened successfully!", link_src);
    let mut buf = [0u8; 256];
    loop {
        let size = read(fd, &mut buf) as usize;
        if size == 0 {
            break;
        }
        println!("{}", core::str::from_utf8(&buf[..size]).unwrap());
    }
    close(fd);
}
#[no_mangle]
pub fn main() -> i32 {
    println!("test sys_linkat and sys_unlinkat...");
    println!("call linkat and try to use another name to access filea");
    let ret = linkat(link_target.as_ptr(), link_src.as_ptr());
    if ret == -1 {
        panic!("error when sys_linkat");
    }
    cat();
    println!("test linkat success!");
    println!("\nnow test unlinkat...");
    let ret = unlinkat(link_src.as_ptr());
    if ret == -1 {
        panic!("error when sys_unlinkat...");
    }
    println!("sys_unlinkat success. try to open that file again...");
    cat();
    0
}