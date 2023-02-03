//! Implementation of [`PageTableEntry`] and [`PageTable`].

use super::{frame_alloc, FrameTracker, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;
use super::address::PPN_WIDTH_SV39;
bitflags! {
    /// page table entry flags
    pub struct PTEFlags: u8 {
        const V = 1 << 0;       // valid
        const R = 1 << 1;       // readable
        const W = 1 << 2;       // writable
        const X = 1 << 3;       // execuatable
        const U = 1 << 4;       // user
        const G = 1 << 5;       // global
        const A = 1 << 6;       // accessed
        const D = 1 << 7;       // dirty
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    /// clean the entry
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << PPN_WIDTH_SV39) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// page table structure. `root_ppn` and vec of `FrameTracker`.
/// offering the `vpn -> ppn` translatation
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
/// note: the pagetable itself consumes a frame's space
impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],    // the only element in vec
            //frames: Vec::new()    // bug... frame will be auto dropped after this fn ends
        }
    }
    /// 在多级页表找到一个虚拟页号对应的页表项的可变引用。
    /// 如果在遍历的过程中发现有节点尚未创建则会新建一个节点。
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                // note: only flag V is set. and i < 2 at this time.
                // this means that the pte points to a lower level pagetable
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    /// 当找不到合法叶子节点的时候不会新建叶子节点,而是直接返回
    /// None 即查找失败。因此，它不会尝试对页表本身进行修改
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    #[allow(unused)]
    /// `set up` a pte. the function's name may be confusing...
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    #[allow(unused)]
    /// clear a pte
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    /// get the contents of a pte
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    /// Temporarily used to get arguments from user space.  
    /// 当遇到需要查一个特定页表（非当前正处在的地址空间的页表时）,
    /// 便可先通过`PageTable::from_token`新建一个页表，
    /// 再调用它的`translate`方法查页表。
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << PPN_WIDTH_SV39) - 1)),
            frames: Vec::new(),
        }
    }
    /// 8usize << 60 | self.root_ppn.0
    /// 按照 satp CSR 格式要求 构造一个无符号 64 位无符号整数，
    /// 使得其分页模式为 SV39 ，且将当前多级页表的根节点所在的物理页号填充进去
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

/// translate a pointer to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        // va -> usize
        start = end_va.into();
    }
    v
}

#[allow(unused)]
fn _vmprint(ppn: PhysPageNum, level: usize){
    let ptes = ppn.get_pte_array();
    for i in 0..512 {
        let pte = ptes[i];
        if pte.is_valid(){
            (0..level + 1).for_each(|_|{print!(".. ");});
            println!("{:<3}: pte: {:x} pa: {:x}", i, pte.bits, pte.ppn().0 << 12);
            // the pte points to a lower level pagetable ??
            if level < 2 && !pte.writable()  && !pte.readable() && !pte.executable(){
                _vmprint(pte.ppn(), level + 1);
            }
        }
    }
}

#[allow(unused)]
/// print a pagetable
pub fn vmprint(pagetable: &PageTable) {
    println!("pagetable: {:x}", usize::from(pagetable.root_ppn) << 12);
    _vmprint(pagetable.root_ppn, 0);
}