//!声明块设备抽象接口 BlockDevice，需要库的使用者提供其实现
use core::any::Any;
/// Trait for block devices(here refers to our file system)
/// which reads and writes data in the unit of blocks
pub trait BlockDevice: Send + Sync + Any {
    ///Read data form block to buffer
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    ///Write data from buffer to block
    fn write_block(&self, block_id: usize, buf: &[u8]);
}
