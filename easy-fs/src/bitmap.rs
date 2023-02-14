use super::{get_block_cache, BlockDevice, BLOCK_SZ};
use alloc::sync::Arc;
/// A bitmap block. 8 * 64 * 8 = 4096(bits)
type BitmapBlock = [u64; 64];
/// Number of bits in a block
const BLOCK_BITS: usize = BLOCK_SZ * 8;
/// A bitmap. 注意 Bitmap 自身是驻留在内存中的(?), 但是它能够表示索引节点/数据块区域
/// 中的那些磁盘块的`分配情况`
/// 磁盘块上位图区域的数据则是要以磁盘数据结构 BitmapBlock 的格式进行操作
pub struct Bitmap {
    start_block_id: usize,
    /// length, number of blocks
    n_blocks: usize,
}

/*
在 easy-fs 布局中存在两类不同的位图，分别对索引节点和数据块进行管理。
每个位图都由若干个块组成，每个块大小为 512 bytes，即 4096 bits。
每个 bit 都代表一个索引节点/数据块的分配状态，0 意味着未分配，而 1 则意味着已经分配出去。
位图所要做的事情是通过基于 bit 为单位的分配
（寻找一个为 0 的bit位并设置为 1）和回收（将bit位清零）来进行索引节点/数据块的分配和回收。
*/

/// Decompose bits into (block_pos, bits64_pos, inner_pos)
fn decomposition(mut bit: usize) -> (usize, usize, usize) {
    let block_pos = bit / BLOCK_BITS;
    bit %= BLOCK_BITS;
    (block_pos, bit / 64, bit % 64)
}

impl Bitmap {
    /// A new bitmap from start block id and number of blocks
    pub fn new(start_block_id: usize, n_blocks: usize) -> Self {
        Self {
            start_block_id,
            n_blocks,
        }
    }
    /// Allocate a new block from a block device. 返回分配的bit编号
    /// note: the returned bit number is also that block's number
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_id in 0..self.n_blocks {
            let pos = get_block_cache(              // get a block from the bitmap and check its bits to do allocation
                block_id + self.start_block_id as usize,
                Arc::clone(block_device),
            )
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                // note: little endian. trailing_ones() is counting from lsb to msb, and msb is stored at low address
                if let Some((bits64_pos, inner_pos)) = bitmap_block
                    .iter()
                    .enumerate()
                    .find(|(_bits64_pos, bits64)| **bits64 != u64::MAX)
                    .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))
                {
                    // modify cache
                    bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                    Some(block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos as usize)
                } else {
                    None
                }
            });
            // 提前返回
            if pos.is_some() {
                return pos;
            }
        }
        None
    }
    /// Deallocate a block
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = decomposition(bit);
        get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device))
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                // that bit must be allocated before
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
                bitmap_block[bits64_pos] -= 1u64 << inner_pos;
            });
    }
    /// Get the max number of allocatable blocks
    pub fn size(&self) -> usize {
        self.n_blocks * BLOCK_BITS
    }
}
