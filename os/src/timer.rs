//! RISC-V timer-related functionality

use crate::config::{CLOCK_FREQ, MAX_APP_NUM};
use crate::sbi::set_timer;
use riscv::register::time;

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;

pub fn get_time() -> usize {
    time::read()
}

/// get current time in microseconds
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

/// set the next timer interrupt. 10ms
pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

// 0 U, 1 K
pub static mut APP_RUNTIME_CNT: [(usize, usize); MAX_APP_NUM] = [(0, 0); MAX_APP_NUM];

/// read out the specified app's runtime in kernel space
pub fn get_kcnt(id: usize) -> usize{
    unsafe {APP_RUNTIME_CNT[id].1}
}
/// read out the specified app's runtime in user space
pub fn get_ucnt(id: usize) -> usize {
    unsafe {APP_RUNTIME_CNT[id].0}
}

