//!提供虚拟文件系统的核心抽象，即索引节点 Inode
//!服务于文件相关系统调用的索引节点层的代码
use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
/// Virtual filesystem layer over easy-fs. easier to use than diskinode.
/// note: DiskInode is the true data structure on the disk.
/// the struct `Inode` below just records a DiskInode's location.
/// and we defien some functions above it to make DiskInode easier to use.
/// (a block can contain 4 inodes)
pub struct Inode {
    ///
    pub block_id: usize,
    ///
    pub block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// overwritting this inode with the target one. used in sys_linkat
    pub fn linkat(&mut self, target: &Arc<Inode>) {
        let binding = get_block_cache(target.block_id, Arc::clone(&self.block_device));
        let binding = binding.lock();
        let target: &DiskInode = binding.get_ref(target.block_offset);
        let target = target as *const DiskInode as *const u8;
        drop(binding);      // otherwise the next `get_block_cache` call will fall into dead loop
        get_block_cache(self.block_id,  Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, |src: &mut DiskInode| {
                let src = (src as *mut DiskInode) as *mut u8;
                unsafe {
                    src.copy_from(target, core::mem::size_of::<DiskInode>());
                }
            });
    }
    /// search the dirents under root inode to find a match,
    /// the dirent tells which inode out file located at.
    /// the 3rd arg seems to be redundant? its always the root inde since we only have one-level directory tree
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_number() as u32);
            }
        }
        None
    }
    /// Find inode under current inode by name
    /// find the specified file's inode accoring to its name
    /// 只会被根目录 Inode 调用
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        // get the root inode from disk(or cache)
        self.read_disk_inode(|disk_inode| {
            // then find the target file's inode number using root inode and its file name
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                // since inode number is known, we can calculate its block id and offset now
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }
    /// Increase the size of a disk inode. what it does:
    /// 1. directly return if new size < old size
    /// 2. otherwise allocate new bits from bitmap and then collect
    ///     those block number into inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
    /// Create inode under current inode by name
    /// 在根目录下创建一个文件，只有根目录的 Inode 会调用()
    /// 1. add a dirent to root inode and increase its size by 32
    /// 2. initialize the dirent and its corresponding inode
    //pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
    pub fn create(&self, name: &str) -> Option<Inode> {
        let mut fs = self.fs.lock();
        // return the given inode's id in disk
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            // already created
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.init(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // bad methods. we can only add dirent at the end of queue
            // append file in the dirent
            let old_size = root_inode.size;
            let new_size = old_size + 32;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                old_size as usize,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        ))
        // release efs lock automatically by compiler
    }
    /// List inodes under current inode
    /// 只有根目录的 Inode 才会调用
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    /// automatically increase size and allocate new inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode
    /// 1. dealloc and return all the relevent blocks back to disk,
    ///     including data blocks and indirect1/2
    /// 2. modify bitmap
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
    /// remove the specified inode(indexed by name) from fs.
    /// can only be called by root inode. steps:
    /// 1. clear the inode block
    /// 2. remove the dirent
    pub fn unlink(&self, name: &str) {
        self.find(name).unwrap().clear();
        // find the dirent and clear it
        self.modify_disk_inode(| root |{
            let n = root.size as usize / DIRENT_SZ;
            let mut offset = 0;
            for _ in 0..n {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    root.read_at(offset, dirent.as_bytes_mut(), &self.block_device),
                    DIRENT_SZ,
                );
                if dirent.name() == name {
                    root.write_at(offset, &[0; DIRENT_SZ], &self.block_device);
                    //root.size -= 32;
                    break;
                }
                offset += DIRENT_SZ;
            }
        });
    }
    /// get the inode's size
    pub fn size(&self) -> u32 {
        get_block_cache(self.block_id, Arc::clone(&self.block_device)).lock()
            .read(self.block_offset, | diskinode: &DiskInode | {
                diskinode.size
            })
    }
    /// true if type == file
    pub fn is_file(&self) -> bool {
        get_block_cache(self.block_id, Arc::clone(&self.block_device)).lock()
            .read(self.block_offset, | diskinode: &DiskInode | {
                diskinode.is_file()
            })
    }
}
