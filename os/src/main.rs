//! The main module and entrypoint
//!
//! Various facilities of the kernels are implemented as submodules. The most
//! important ones are:
//!
//! - [`trap`]: Handles all cases of switching from userspace to the kernel
//! - [`syscall`]: System call handling and implementation
//!
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality. (See its source code for
//! details.)
//!
//! We then call [`batch::run_next_app()`] and for the first time go to
//! userspace.

#![deny(missing_docs)]
#![deny(warnings)]
#![allow(unused)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::arch::{global_asm, asm};

#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
pub mod batch;
mod lang_items;
mod sbi;
mod sync;
pub mod syscall;
pub mod trap;

global_asm!(include_str!("entry.asm"));     // 0x80200000
global_asm!(include_str!("link_app.S"));    // 0x80400000

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
/*  another implementation:
    (sbss as usize..ebss as usize).for_each(
        |a| unsafe { (a as *mut u8).write_volatile(0); }
    );
*/
}

fn welcome() {
    // here we are "cheating" the compiler. We ask it to help us find functions.
    // While in fact we treated these "functions" as values
    extern "C" {
        fn stext();     // begin addr of text segment
        fn etext();     // end addr of text segment
        fn srodata();   // start addr of Read-Only data segment
        fn erodata();   // end addr of Read-Only data ssegment
        fn sdata();     // start addr of data segment
        fn edata();     // end addr of data segment
        fn sbss();      // start addr of BSS segment
        fn ebss();      // end addr of BSS segment
        fn boot_stack_lower_bound(); // stack lower bound
        fn boot_stack_top(); // stack top
    }
    info!("memory layout:");
    info!("rust-sbi  [0x80000000, 0x80200000]");
    info!(".text     [{:#x}, {:#x})", stext as usize, etext as usize);
    info!(".rodata   [{:#x}, {:#x})", srodata as usize, erodata as usize);
    info!(".data     [{:#x}, {:#x})", sdata as usize, edata as usize);
    info!(".stack    [{:#x}, {:#x}]",
        boot_stack_lower_bound as usize, boot_stack_top as usize);
    info!(".bss      [{:#x}, {:#x})", sbss as usize, ebss as usize);
}

/// the rust entry-point of os
#[no_mangle]
pub fn rust_main() -> ! {
    unsafe{ asm!("li t0, 0x114","li t1, 0x514");};
    clear_bss();
    println!("\x1b[42mHello world!\x1b[0m");
    welcome();
    //(0..110).for_each(|n| { print!("\x1b[{}m {} \x1b[0m ", n, n);});
    println!("");
    trap::init();           // set up stvec
    batch::init();          // in fact just print some infomation about app
    batch::run_next_app();
}
