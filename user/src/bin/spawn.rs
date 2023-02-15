#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::spawn;

#[no_mangle]
pub fn main() -> i32 {
    println!("test spawn...");
    println!("a process called 'ls' is added to the ready queue, and should be execuated later");
    let ret = spawn("ls\0".as_ptr());
    println!("spawn finished. continue to run main...");
    ret as i32
}