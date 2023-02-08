//! The main module and entrypoint
//!
//! Various facilities of the kernels are implemented as submodules. The most
//! important ones are:
//!
//! - [`trap`]: Handles all cases of switching from userspace to the kernel
//! - [`task`]: Task management
//! - [`syscall`]: System call handling and implementation
//! - [`mm`]: Address map using SV39
//! - [`sync`]:Wrap a static data structure inside it so that we are able to access it without any `unsafe`.
//!
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality. (See its source code for
//! details.)
//!
//! We then call [`task::run_tasks()`] and for the first time go to
//! userspace.

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

//use crate::mm::{vmprint, KERNEL_SPACE};

extern crate alloc;

#[macro_use]
extern crate bitflags;

#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
mod config;
mod lang_items;
mod loader;
pub mod mm;
mod sbi;
pub mod sync;
pub mod syscall;
pub mod task;
mod timer;
pub mod trap;

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));
/// clear BSS segment
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}
macro_rules! color_text {
    ($text:expr, $color:expr) => {{
        format_args!("\x1b[{}m{}\x1b[0m", $color, $text)
    }};
}
fn init() {
    clear_bss();
    mm::init();
    task::add_initproc();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    loader::list_apps();    
}

#[no_mangle]
/// the rust entry-point of os
pub fn rust_main() -> ! {
    init();
    println!(
        "{}{}{}{}{} {}{}{}{} {}{}{}{}{}{}",
        color_text!("H", 31),
        color_text!("e", 32),
        color_text!("l", 33),
        color_text!("l", 34),
        color_text!("o", 35),
        color_text!("R", 36),
        color_text!("u", 37),
        color_text!("s", 90),
        color_text!("t", 91),
        color_text!("u", 92),
        color_text!("C", 93),
        color_text!("o", 94),
        color_text!("r", 95),
        color_text!("e", 96),
        color_text!("!", 97),
    );
    debug!(" start to run initproc!");
    task::run_tasks();
    panic!("Unreachable in rust_main!");
}
