#![no_std]
#![no_main]

use user_lib::ls;

#[no_mangle]
pub fn main() -> i32 {
    ls() as i32
}
