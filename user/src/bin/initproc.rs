#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, wait, yield_};

#[no_mangle]
fn main() -> i32 {
    println!("[initproc] fork");
    if fork() == 0 {
        println!("[initproc] child: exec user_shell");
        exec("user_shell\0");
    } else {
        // parent
        loop {
            let mut exit_code: i32 = 0;
            let pid = wait(&mut exit_code);
            // does not exist any exited process
            if pid == -1 {
                yield_();
                continue;
            }
            // capture a exited task
            println!(
                "[initproc] Released a zombie process, pid={}, exit_code={}",
                pid, exit_code,
            );
        }
    }
    0
}
