//! Implementation of [`FrameAllocator`] which
//! controls all the frames in the operating system.

use super::{PhysAddr, PhysPageNum};
use crate::config::MEMORY_END;
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::*;

/// use a `ppn` to manage a frame which has the same lifecycle as the tracker.
/// a simple wrapper of `ppn`. use `RAII` to manage resources
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    /// wrap a `ppn` into a `FrameTracker`
    pub fn new(ppn: PhysPageNum) -> Self {
        // I move the page-clean's job from alloc time to dealloc time
        Self { ppn }
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

/// manage page frames. a set of functions
trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// an implementation for frame allocator. based on `stack-style`, because those 
/// first recycled pages will also firstly be reused. the `global frame allocator`.
/// a page could be at 1 of the following 3 states:
/// 1. ppn between current and end: `not touched yet`. nobody had used them before.
///     when allocating new pages and no recycled left, we would pick a new page from here.
///     then that page would never come back to state 1 and either at state 2 or 3
/// 2. `recycled`: have been put into use before, but now deallocated and not be used by anyone. 
///     when allocating new pages, we would first refer to those pages
/// 3. not above: those pages are currently `in use` by someone
pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
        self.recycled = Vec::new();
    }
}
impl FrameAllocator for StackFrameAllocator {
    /// returns an empty allocator, don't use before initialized
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }
    fn alloc(&mut self) -> Option<PhysPageNum> {
        // use recyled first
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        }
        // pick a new page. question:  `<=` or `<` ?
        // note: current and end's types are both usize,
        // but in fact they hold some meaning of ppn
        else if self.current < self.end {
            self.current += 1;
            Some(PhysPageNum(self.current - 1))
        } else {
            None
        }
    }
    /// free and clean a page
    fn dealloc(&mut self, ppn: PhysPageNum) {
        // validity check
        if ppn.0 >= self.current || self.recycled.iter().any(|&v| v == ppn.0) {
            panic!("Frame ppn={:#x} has not been allocated!", ppn.0);
        }
        let page_area = ppn.get_bytes_array();
        for i in page_area {
            *i = 0;
        }
        // recycle
        self.recycled.push(ppn.0);
    }
}

lazy_static! {
    /// frame allocator instance through lazy_static!
    /// allocate and deallocate physical `pages`. 
    /// manage through `page number`
    pub static ref FRAME_ALLOCATOR: UPSafeCell<StackFrameAllocator> =
        unsafe { UPSafeCell::new(StackFrameAllocator::new()) };
}

/// initiate the frame allocator using `ekernel` and `MEMORY_END`
pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    let first_page_number = PhysAddr::from(ekernel as usize).ceil();
    let last_page_number = PhysAddr::from(MEMORY_END).floor();
    let n = last_page_number.0 - first_page_number.0;
    // using page number. forms a ppn from that pa
    FRAME_ALLOCATOR.exclusive_access().init(
        first_page_number,
        last_page_number
    );
    debug!(" ekernel = {:x}, 1st ppn = {:x}, last ppn = {:x}. #pages = {:x} ({})", 
        ekernel as usize, first_page_number.0, last_page_number.0, n, n);
    //panic!("test");
}

/// allocate a frame, return the ppn of that frame
pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
}

/// deallocate a frame
fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

#[allow(unused)]
/// a simple test for frame allocator
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}
