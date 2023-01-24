#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    //taskid();
    let id = user_lib::taskid();
    println!("taskid = {}", id);
    0
}
