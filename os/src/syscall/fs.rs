//! File and filesystem-related syscalls

const FD_STDOUT: usize = 1;

use crate::batch::{APP_BASE_ADDRESS, APP_SIZE_LIMIT, USER_STACK_SIZE, get_user_sp};

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            // safety check
            let ptr = buf as usize;
            let sp = get_user_sp();
            // info!(" buf as usize: {:x}, sp: {:x}", ptr, sp);
            if ( ptr >= sp - USER_STACK_SIZE) && ( ptr + len <= sp) || // inside the stack
                ( ptr + len <= APP_SIZE_LIMIT + APP_BASE_ADDRESS ) && ( ptr >= APP_BASE_ADDRESS){
                let slice = unsafe { core::slice::from_raw_parts(buf, len) };
                let str = core::str::from_utf8(slice).unwrap();
                print!("{}", str);
                len as isize
            }
            else {
                -1 as isize
            }
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}
