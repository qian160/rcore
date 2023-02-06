//! Implementation of physical and virtual address and page number.
use super::PageTableEntry;
use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};
use core::fmt::{self, Debug, Formatter};

/// ----- some comments -----
/// the name 'page number' is a little confusing...
/// in fact, they do tell us information about "which page [that address] belongs to"
/// for L0-pagetable, [that address] exactly refers to the data.
/// but for L1 and L2, it refers to next-level's pagetables
/// 
/// about vpn:
/// ppn is nature and easy to understand, since physicaly address is divided into pages
/// but what about vpn? in fact when seeing a virtual address,
/// we know that it must belongs to some physical page. the case here is abstraction.
/// that is, a program should not be aware of the existance virtual memory
/// for example, a program sees an address of 0x80600000, and thought its page number was 0x80600.
/// however after translation it may be mapped to page 0x80400
/// what we see is different from what we get, so we call it virtual (page number)


/// physical address
pub const PA_WIDTH_SV39: usize = 56;
pub const VA_WIDTH_SV39: usize = 39;
#[allow(unused)]
pub const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;
#[allow(unused)]
pub const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - PAGE_SIZE_BITS;

/// physical address. `56`bits, 44 + 12
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);
/// virtual address. `39`bits
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);
/// physical page number, `44`bits
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);
/// virtual page number. `27`bits consisted of 3 9-bit indexes
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtPageNum(pub usize);

/// Debugging

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VA:{:#x}", self.0))
    }
}
impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VPN:{:#x}", self.0))
    }
}
impl Debug for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PA:{:#x}", self.0))
    }
}
impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PPN:{:#x}", self.0))
    }
}

/// T: {PhysAddr, VirtAddr, PhysPageNum, VirtPageNum}
/// T -> usize: T.0
/// usize -> T: usize.into()

/// these functions below are all about just `getting the lower bits`...

impl From<usize> for PhysAddr {
    /// returrn the lower `56` bits
    fn from(va: usize) -> Self {
        Self(va & ((1 << PA_WIDTH_SV39) - 1))
    }
}

// this usize -> ppn transform is not so good. for
// convenience reasons we would sometimes wish the 
// input to be an address, while the other times
// a page number. here i just satisfy both. and the
// rule is easy: if input < ekernel, then  it is
// regarded as a page number. otherwise an addresss
// if you want to modify, pay atten to the following functions:
// 1. frame_allocator.rs: alloc(),
// 2. pagetable.rs: ppn(), from_token()

// but in some places it will also be convenient
// if usize could be a page number. so here I write 2 usages
impl From<usize> for PhysPageNum {
    /// `usize` -> `pa`(56 bits) -> `ppn`(>>12)
    fn from(v: usize) -> Self {
        extern "C" {
            fn ekernel();
        }
        if v >= VirtAddr::from(ekernel as usize).0 {
            Self((v & ((1 << PA_WIDTH_SV39) - 1)) >> PAGE_SIZE_BITS)
        }
        else {
            PhysPageNum(v)
        }
//        Self(va & ((1 << PA_WIDTH_SV39) - 1))
    }
}
impl From<usize> for VirtAddr {
    /// return the lower `39` bits
    fn from(addr: usize) -> Self {
        Self(addr & ((1 << VA_WIDTH_SV39) - 1))
    }
}
// here we can't detect whether the input is a page number
// or an address. so just force it to be an address
impl From<usize> for VirtPageNum {
    /// `usize` -> `va`(39 bits) -> `vpn`(>>12)
    /// note: the usize arg must be an `address`, not pagenumber
    /// we could also use the struct's construction function like:
    /// VirtPageNum::from(0x1000) === VirtPageNum(0x1)
    fn from(addr: usize) -> Self {
//        Self(va & ((1 << VPN_WIDTH_SV39) - 1))
        Self((addr & ((1 << VA_WIDTH_SV39) - 1)) >> PAGE_SIZE_BITS)
    }
}
impl From<PhysAddr> for usize {
    /// just get the struct's member
    fn from(v: PhysAddr) -> Self {
        v.0
    }
}
impl From<PhysPageNum> for usize {
    /// just get the struct's member
    fn from(v: PhysPageNum) -> Self {
        v.0
    }
}
// this is required by the docs.
/* SV39 分页模式规定 64 位虚拟地址的[63: 39]这 25 位必须和第 38 位相同，否则MMU会直接认定它是
一个不合法的虚拟地址。通过这个检查之后 MMU再取出低39位尝试将其转化为一个 56 位的物理地址。*/
impl From<VirtAddr> for usize {
    /// va -> uszie. note: va must meet some requirments
    fn from(v: VirtAddr) -> Self {
        if v.0 >= (1 << (VA_WIDTH_SV39 - 1)) {
            // 0000 1000...0        1 << 39. 39 0s after 1
            // 0000 0111...1        - 1
            // 1111 1000...0        neg
            v.0 | (!((1 << VA_WIDTH_SV39) - 1))
        } else {
            v.0
        }
    }
}
impl From<VirtPageNum> for usize {
    fn from(v: VirtPageNum) -> Self {
        v.0
    }
}
///
impl VirtAddr {
    /// va -> vpn `0x100_001 -> 0x100`
    pub fn floor(&self) -> VirtPageNum {
        VirtPageNum(self.0 / PAGE_SIZE)
    }
    /// va -> vpn `0x100_001 -> 0x101`
    pub fn ceil(&self) -> VirtPageNum {
        VirtPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
    }
    /// Get page offset (lower `12` bits)
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    /// Check page aligned
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}
impl From<VirtAddr> for VirtPageNum {
    /// va -> vpn. use floor()
    fn from(v: VirtAddr) -> Self {
        assert_eq!(v.page_offset(), 0);     // ???
        v.floor()
    }
}
impl From<VirtPageNum> for VirtAddr {
    /// `left shift 12 bits`. the starting address of that page
    fn from(v: VirtPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
}
impl PhysAddr {
    /// tells which `ppn` that `pa` belongs to
    /// note: `0x10_001 -> 0x10`
    pub fn floor(&self) -> PhysPageNum {
        PhysPageNum(self.0 / PAGE_SIZE)
    }
    /// tells which `ppn` that `pa` belongs to
    /// note: `0x10_001 -> 0x11`
    pub fn ceil(&self) -> PhysPageNum {
        PhysPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
    }
    /// Get page offset. lower `12` bits
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    /// Check page aligned
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}
impl From<PhysAddr> for PhysPageNum {
    fn from(v: PhysAddr) -> Self {
        assert_eq!(v.page_offset(), 0);     // ???
        v.floor()
    }
}
impl From<PhysPageNum> for PhysAddr {
    /// `left shift 12 bits`. the starting address of that page
    fn from(v: PhysPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
}

impl VirtPageNum {
    ///Return VPN 3 level index
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        for i in (0..3).rev() {
            idx[i] = vpn & 511;
            vpn >>= 9;
        }
        idx
    }
}

impl PhysAddr {
    ///Get mutable reference to `PhysAddr` value
    pub fn get_mut<T>(&self) -> &'static mut T {
        unsafe { (self.0 as *mut T).as_mut().unwrap() }
    }
}
impl PhysPageNum {
    ///Get `PageTableEntry` on `PhysPageNum`
    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512) }
    }
    ///
    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096) }
    }
    ///
    pub fn get_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = (*self).into();
        pa.get_mut()
    }
}

pub trait StepByOne {
    fn step(&mut self);
}
impl StepByOne for VirtPageNum {
    /// move to next page. `self.0 += 1`
    /// (just increases the struct member)
    fn step(&mut self) {
        self.0 += 1;
    }
}

#[derive(Copy, Clone)]
/// a simple range structure for type T. [l, r)
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    l: T,
    r: T,
}
impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self { l: start, r: end }
    }
    pub fn get_start(&self) -> T {
        self.l
    }
    pub fn get_end(&self) -> T {
        self.r
    }
    #[allow(unused)]
    pub fn contain(&self, val: T) -> bool {
        val >= self.l && val < self.r
    }
}
impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    type IntoIter = SimpleRangeIterator<T>;
    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.l, self.r)
    }
}
/// iterator for the simple range structure
pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}
impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        Self { current: l, end: r }
    }
}
impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.step();
            Some(t)
        }
    }
}
/// a simple range structure for virtual page number
/// 描述一段虚拟页号的连续区间
pub type VPNRange = SimpleRange<VirtPageNum>;
