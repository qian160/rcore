#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

fn test() {
    println!("level 1");
    user_lib::trace();
    foo();
}

fn foo() {
    println!("level 2");
    user_lib::trace();
    bar();
}

fn bar() {
    println!("level 3");
    user_lib::trace();
}
#[no_mangle]
fn main() -> i32 {
    println!("init");
    user_lib::trace();
    test();
    0
}
