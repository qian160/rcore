//! The main module and entrypoint
//!
//! Various facilities of the kernels are implemented as submodules. The most
//! important ones are:
//!
//! - [`trap`]: Handles all cases of switching from userspace to the kernel
//! - [`task`]: Task management
//! - [`syscall`]: System call handling and implementation
//!
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality. (See its source code for
//! details.)
//!
//! We then call [`task::run_first_task()`] and for the first time go to
//! userspace.

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

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
mod mm;
mod sbi;
mod sync;
pub mod syscall;
pub mod task;
mod timer;
pub mod trap;

core::arch::global_asm!(include_str!("entry.asm"));     // 0x80200000
core::arch::global_asm!(include_str!("link_app.S"));    // 0x80400000

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
        fn stext();     // start addr of text segment
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
    info!("rust-sbi  [0x80000000, 0x80200000)");
    info!(".text     [{:#x}, {:#x})", stext as usize, etext as usize);
    info!(".rodata   [{:#x}, {:#x})", srodata as usize, erodata as usize);
    info!(".data     [{:#x}, {:#x})", sdata as usize, edata as usize);
    info!(".stack    [{:#x}, {:#x})",
        boot_stack_lower_bound as usize, boot_stack_top as usize);
    info!(".bss      [{:#x}, {:#x})", sbss as usize, ebss as usize);
    info!("😄Hello world😄");
}

fn init() {
    mm::init();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
}

#[no_mangle]
/// the rust entry-point of os
pub fn rust_main() -> ! {
    clear_bss();
    welcome();
    init();
    task::run_first_task();
    panic!("Unreachable in rust_main!");
}
