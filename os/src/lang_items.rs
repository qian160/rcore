//! The panic handler
use crate::sbi::shutdown;
use core::{panic::PanicInfo};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        error!(
            "[kernel] Panicked at {}:{} {} 😱",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        error!("[kernel] Panicked: {} 😱", info.message().unwrap());
    }
    unsafe{
        trace();
    }
    shutdown()
}

/*
risc-v stack frame:

-------------------- high (fp)
*   return address
*   prev fp
    saved registers
    local variables
    ...
-------------------- low (sp)
*/
/// print stack frame
pub unsafe fn trace(){
    let mut fp: *const usize;
    core::arch::asm!("mv {}, fp", out(reg) fp);
    println!("\t\t== Begin stack trace ==");
    while fp != core::ptr::null() {
        let saved_ra = *fp.sub(1);
        let saved_fp = *fp.sub(2);

        println!("ra = 0x{:016x}, prev fp = 0x{:016x}", saved_ra, saved_fp);

        fp = saved_fp as *const usize;
    }
    println!("\t\t== End stack trace ==");
}