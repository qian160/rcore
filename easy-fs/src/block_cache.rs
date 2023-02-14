//!块缓存层，将块设备中的部分块缓存在内存中
use super::{BlockDevice, BLOCK_SZ};
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
use spin::Mutex;
/// Cached block inside memory
pub struct BlockCache {
    /// cached block data
    cache: [u8; BLOCK_SZ],
    /// underlying block id
    block_id: usize,
    /// underlying block device
    /// 一个底层块设备的引用，可通过它进行块读写
    block_device: Arc<dyn BlockDevice>,
    /// whether the block is dirty
    modified: bool,
}

impl BlockCache {
    /// Load a new BlockCache from disk.
    /// 当我们创建一个 BlockCache 的时候，这将触发一次 read_block 将一个块上的数据从磁盘读到缓冲区 cache
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut cache = [0u8; BLOCK_SZ];
        block_device.read_block(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }
    /// Get the address of an offset inside the cached block data
    fn addr_of_offset(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }
    /// get the referrence of the data inside blockcache at certain offset
    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) }
    }
    /// get the mut referrence of the data inside blockcache at certain offset
    /// note: for safety reason we set modified = true, 
    /// no matter whether the data will be modified or not later
    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        self.modified = true;
        let addr = self.addr_of_offset(offset);
        unsafe { &mut *(addr as *mut T) }
    }

    /* get_ref/get_mut 进一步封装为更为易用的形式 -> read/modify */

    /// 在 BlockCache 缓冲区偏移量为 offset 的位置获取一个类型为 T 的
    /// 磁盘上数据结构的`不可变`引用并让它执行传入的闭包 f 中所定义的操作。
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }
    /// 在 BlockCache 缓冲区偏移量为 offset 的位置获取一个类型为 T 的
    /// 磁盘上数据结构的`可变`引用，并让它执行传入的闭包 f 中所定义的操作。
    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }

    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device.write_block(self.block_id, &self.cache);
        }
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync()
    }
}
/// Use a block cache of 16 blocks
const BLOCK_CACHE_SIZE: usize = 16;

pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}
/// 块缓存全局管理器：当我们要对一个磁盘块进行读写时，首先看它是否已经被载入到内存缓存中了，
/// 如果已经被载入的话则直接返回，否则需要先读取磁盘块的数据到内存缓存中。
/// 此时，如果内存中驻留的磁盘块缓冲区的数量已满，则需要遵循某种缓存替换算法将某个块的缓存从内存中移除，再将刚刚读到的块数据加入到内存缓存中
impl BlockCacheManager {
    /// create an empty manager
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
    /// 尝试从块缓存管理器中获取一个编号为 block_id 的块的块缓存.
    /// 如果找不到，会从磁盘读取到内存中，还有可能会发生缓存替换
    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some(pair) = self.queue.iter().find(|pair| pair.0 == block_id) {
            Arc::clone(&pair.1)
        } else {
            // substitute
            // 管理器保存的块缓存数量是否已经达到了上限。
            // 如果达到了上限才需要执行缓存替换算法，丢掉某个块缓存并空出一个空位
            // (从队头遍历到队尾找到第一个强引用计数恰好为 1 的块缓存并将其替换出去。)
            if self.queue.len() == BLOCK_CACHE_SIZE {
                // from front to tail
                if let Some((idx, _)) = self
                    .queue
                    .iter()
                    .enumerate()
                    .find(|(_, pair)| Arc::strong_count(&pair.1) == 1)
                {
                    self.queue.drain(idx..=idx);
                } else {
                    panic!("Run out of BlockCache!");
                }
            }
            // load block into mem and push back. this will trigger a read
            let block_cache = Arc::new(Mutex::new(BlockCache::new(
                block_id,
                Arc::clone(&block_device),
            )));
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

lazy_static! {
    /// The global block cache manager
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> =
        Mutex::new(BlockCacheManager::new());
}
/// Get the block cache corresponding to the given block id and block device
/// 尝试从块缓存管理器中获取一个编号为 block_id 的块的块缓存，
/// 如果找不到，会从磁盘读取到内存中，还有可能会发生缓存替换
pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER
        .lock()
        .get_block_cache(block_id, block_device)
}
/// Sync all block cache to block device
pub fn block_cache_sync_all() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in manager.queue.iter() {
        cache.lock().sync();
    }
}
