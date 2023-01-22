#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

enum State{
    LessThanOne, One, MoreThanOne
}

static mut CNT: u32 = 0;

macro_rules! CurrentState{
    ($level: expr) => {
        if      $level <  1 {State::LessThanOne}
        else if $level == 1 {State::One}
        else    {State::MoreThanOne}
    }
}

fn hanoi(level: u32, from: char, by: char, to: char) -> ()
{
    match CurrentState!(level){
        State::LessThanOne  =>  {
            panic!("should not happen");
        },
        State::One  =>  {
            unsafe{
                CNT += 1;
                println!(" [{:>2}]  {} -> {}", CNT, from, to);
            }
        },
        State::MoreThanOne  =>  {
            hanoi(level - 1, from, to, by);
            hanoi(1, from, by, to);
            hanoi(level - 1, by, from, to);
        }
    };
}

#[no_mangle]
fn main() -> i32 {
    println!("test hanoi tower of level 5");
    hanoi(5, 'a', 'b', 'c');
    unsafe {(CNT != 31) as i32}
}
