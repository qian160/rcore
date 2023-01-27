#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{get_time, yield_, taskinfo, init_task_info};

#[no_mangle]
fn main() -> i32 {
    let current_timer = get_time();
    let wait_for = current_timer + 3000;
    while get_time() < wait_for {
        yield_();
    }
    println!("Test sleep OK!");
    println!("now test taskinfo");
    let mut info = init_task_info();
    (0..20).for_each(|i| {
        match taskinfo(i, core::ptr::addr_of_mut!(info)) {
            -1 => {
                println!("bad task id{}", i);
            },
            _ => {
                println!("{:?}",info);
            }
        }       
    });
    println!("sys_taskinfo OK!");
    0
}
