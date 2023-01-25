//! Uniprocessor interior mutability primitives

use core::cell::{RefCell, RefMut};

/// Wrap a static data structure inside it so that we are
/// able to access it without any `unsafe`.
///
/// We should only use it in uniprocessor.
///
/// In order to get mutable reference of inner data, call
/// `exclusive_access`.

// Why not just use static mut AppManager? the inner data could be muted easily that way
// there are 2 reasons for not using that method:
// 1. this introduces a mutable global variable, any write to it will be 
//     considered unsafe be rust. So we will have to use lots of unsafe blocks
// 2. the logic is not so good in fact. We just want to change its inner field, not
//     the structure itself. If we declare with static mut, then the structure could be assigned to another one
pub struct UPSafeCell<T> {
    /// inner data
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}

impl<T> UPSafeCell<T> {
    /// User is responsible to guarantee that inner struct is only used in
    /// uniprocessor.
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }
    /// Exclusive access inner data in UPSafeCell. Panic if the data has been borrowed.
    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}
