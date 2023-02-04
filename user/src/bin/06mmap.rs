#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::{mmap, munmap};
#[no_mangle]
fn main() -> i32 {
    println!("test mmap...\n");
    println!("Kernel should not kill this application!");
    mmap(0x514000, 100, 3);
    unsafe {
        println!(" before: a = {}", (0x514000 as *const u8).read());
        (0x514000 as usize as *mut u8).write(100);
        println!(" after: a = {}", (0x514000 as *const u8).read());
    }
    munmap(0x514000, 100);
    0
}
