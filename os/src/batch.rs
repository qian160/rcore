//! batch subsystem

use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use core::arch::asm;
use lazy_static::*;
///
pub const USER_STACK_SIZE: usize = 4096 * 2;    // 8KB
///
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
///
pub const MAX_APP_NUM: usize = 16;
///
pub const APP_BASE_ADDRESS: usize = 0x80400000;
///
pub const APP_SIZE_LIMIT: usize = 0x20000;

// pay attention to the code in sys_exit 
/*
    for satety reason, we use 2 seperate stacks. If not so, a user program could easily
    get kernel's information(like some addresses of kernel functions) after returning
    from a trap, which is not so good. Also when a trap happens, a stack switch must be performed
 */
/// before a user program entering traps, normally its GPR's states will be saved to kernel stack
#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
/// userstack
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

/// these 2 stacks are different from the register sp
/// if we directly save things using reg sp, it would be hard to manage and could be dangerous
static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};
static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

/// get userstack's sp. Note: user sp and the reg sp are different things
pub fn get_user_sp() -> usize {
    USER_STACK.get_sp()
}

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        // sub sp by a size_of context. And then fill it with the given argument
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *cx_ptr = cx;
        }
        unsafe { cx_ptr.as_mut().unwrap() }
    }
}

impl UserStack {
    pub fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

/*  保存应用数量和各自的位置信息，以及当前执行到第几个应用了。
    根据应用程序位置信息，初始化好应用所需内存空间，并加载应用执行*/

struct AppManager {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],
}

impl AppManager {
    pub fn print_app_info(&self) {
        info!("[kernel] num_app = {}", self.num_app);
        for i in 0..self.num_app {
            info!(
                "[kernel] app_{} [{:#x}, {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }
    /// copy binary data from the compiled object file to the target address(0x80400000)
    /// note: the os is compiled together with apps
    unsafe fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            info!("All applications completed!");
            use crate::board::QEMUExit;
            crate::board::QEMU_EXIT_HANDLE.exit_success();
        }
        info!("[kernel] Loading app_{}", app_id);
        // clear icache
        asm!("fence.i");
        // clear app area
        core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);
        // find the address of the target app in the binary file. A pointer is returned
        let app_src = core::slice::from_raw_parts(
            self.app_start[app_id] as *const u8,
            self.app_start[app_id + 1] - self.app_start[app_id],
        );
        // the target address for loading the app
        let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
        // copy source data to dest using that pointer
        app_dst.copy_from_slice(app_src);
        // core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len()).copy_from_slice(app_src);
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new({
            extern "C" {
                fn _num_app();
            }
            let num_app_ptr = _num_app as usize as *const usize;
            let num_app = num_app_ptr.read_volatile();
            let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
            let app_start_raw: &[usize] =
                core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1);
            app_start[..=num_app].copy_from_slice(app_start_raw);
            AppManager {
                num_app,
                current_app: 0,
                app_start,
            }
        })
    };
}

/// sys_taskid
pub fn taskid() -> usize{
    APP_MANAGER.exclusive_access().current_app
}

/// init batch subsystem
pub fn init() {
    print_app_info();
}

/// print apps info
pub fn print_app_info() {
    APP_MANAGER.exclusive_access().print_app_info();
}

/// run next app
pub fn run_next_app() -> ! {
    let mut app_manager = APP_MANAGER.exclusive_access();
    let current_app = app_manager.get_current_app();
    unsafe {
        app_manager.load_app(current_app);
    }
    app_manager.move_to_next_app();
    drop(app_manager);
    // before this we have to drop local variables related to resources manually
    // and release the resources
    extern "C" {
        fn __restore(cx_addr: usize);
    }
    unsafe {
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
            APP_BASE_ADDRESS,
            USER_STACK.get_sp(),
        )) as *const _ as usize);
    }
    panic!("Unreachable in batch::run_current_app!");
}
