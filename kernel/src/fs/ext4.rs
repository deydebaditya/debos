//! ext4 Filesystem Driver
//!
//! Implements read support for the ext4 filesystem.
//!
//! ## ext4 Layout
//! - Superblock at offset 1024
//! - Block groups
//! - Inodes and data blocks
//!
//! ## Features Supported
//! - Basic file and directory reading
//! - 64-bit block addressing
//! - Extent-based file allocation
//!
//! ## References
//! - https://ext4.wiki.kernel.org/index.php/Ext4_Disk_Layout

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use spin::Mutex;

use crate::drivers::block;
use super::{FsError, FsResult, InodeType, Stat, DirEntry};

/// ext4 magic number
const EXT4_MAGIC: u16 = 0xEF53;

/// Superblock offset in bytes
const SUPERBLOCK_OFFSET: u64 = 1024;

/// Superblock size in bytes
const SUPERBLOCK_SIZE: usize = 1024;

/// ext4 Superblock
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Superblock {
    /// Total inode count
    pub inodes_count: u32,
    /// Total block count (low 32 bits)
    pub blocks_count_lo: u32,
    /// Reserved blocks count (low 32 bits)
    pub r_blocks_count_lo: u32,
    /// Free blocks count (low 32 bits)
    pub free_blocks_count_lo: u32,
    /// Free inodes count
    pub free_inodes_count: u32,
    /// First data block
    pub first_data_block: u32,
    /// Block size = 1024 << log_block_size
    pub log_block_size: u32,
    /// Cluster size = 1024 << log_cluster_size
    pub log_cluster_size: u32,
    /// Blocks per group
    pub blocks_per_group: u32,
    /// Clusters per group
    pub clusters_per_group: u32,
    /// Inodes per group
    pub inodes_per_group: u32,
    /// Mount time
    pub mtime: u32,
    /// Write time
    pub wtime: u32,
    /// Mount count
    pub mnt_count: u16,
    /// Max mount count
    pub max_mnt_count: u16,
    /// Magic signature (0xEF53)
    pub magic: u16,
    /// Filesystem state
    pub state: u16,
    /// Behavior on errors
    pub errors: u16,
    /// Minor revision level
    pub minor_rev_level: u16,
    /// Last check time
    pub lastcheck: u32,
    /// Check interval
    pub checkinterval: u32,
    /// Creator OS
    pub creator_os: u32,
    /// Revision level
    pub rev_level: u32,
    /// Default UID for reserved blocks
    pub def_resuid: u16,
    /// Default GID for reserved blocks
    pub def_resgid: u16,
    
    // EXT4_DYNAMIC_REV specific
    /// First non-reserved inode
    pub first_ino: u32,
    /// Inode size
    pub inode_size: u16,
    /// Block group number of this superblock
    pub block_group_nr: u16,
    /// Compatible feature set
    pub feature_compat: u32,
    /// Incompatible feature set
    pub feature_incompat: u32,
    /// Read-only compatible feature set
    pub feature_ro_compat: u32,
    /// 128-bit UUID
    pub uuid: [u8; 16],
    /// Volume name
    pub volume_name: [u8; 16],
    /// Directory where last mounted
    pub last_mounted: [u8; 64],
    /// Compression algorithm
    pub algorithm_usage_bitmap: u32,
    
    // Performance hints
    pub prealloc_blocks: u8,
    pub prealloc_dir_blocks: u8,
    pub reserved_gdt_blocks: u16,
    
    // Journaling
    pub journal_uuid: [u8; 16],
    pub journal_inum: u32,
    pub journal_dev: u32,
    pub last_orphan: u32,
    pub hash_seed: [u32; 4],
    pub def_hash_version: u8,
    pub jnl_backup_type: u8,
    pub desc_size: u16,
    pub default_mount_opts: u32,
    pub first_meta_bg: u32,
    pub mkfs_time: u32,
    pub jnl_blocks: [u32; 17],
    
    // 64-bit support
    pub blocks_count_hi: u32,
    pub r_blocks_count_hi: u32,
    pub free_blocks_count_hi: u32,
    pub min_extra_isize: u16,
    pub want_extra_isize: u16,
    pub flags: u32,
    pub raid_stride: u16,
    pub mmp_interval: u16,
    pub mmp_block: u64,
    pub raid_stripe_width: u32,
    pub log_groups_per_flex: u8,
    pub checksum_type: u8,
    pub reserved_pad: u16,
    pub kbytes_written: u64,
    // ... more fields
}

impl Superblock {
    /// Get block size in bytes
    pub fn block_size(&self) -> u32 {
        1024 << self.log_block_size
    }
    
    /// Get total block count (64-bit)
    pub fn blocks_count(&self) -> u64 {
        ((self.blocks_count_hi as u64) << 32) | (self.blocks_count_lo as u64)
    }
    
    /// Get number of block groups
    pub fn block_groups(&self) -> u32 {
        (self.blocks_count_lo + self.blocks_per_group - 1) / self.blocks_per_group
    }
    
    /// Check if valid ext4
    pub fn is_valid(&self) -> bool {
        self.magic == EXT4_MAGIC
    }
    
    /// Get volume name
    pub fn volume_name(&self) -> String {
        let end = self.volume_name.iter()
            .position(|&b| b == 0)
            .unwrap_or(16);
        String::from_utf8_lossy(&self.volume_name[..end]).to_string()
    }
}

/// Block Group Descriptor (32-byte version)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct BlockGroupDesc32 {
    /// Block bitmap location
    pub block_bitmap_lo: u32,
    /// Inode bitmap location
    pub inode_bitmap_lo: u32,
    /// Inode table location
    pub inode_table_lo: u32,
    /// Free blocks count
    pub free_blocks_count_lo: u16,
    /// Free inodes count
    pub free_inodes_count_lo: u16,
    /// Directory count
    pub used_dirs_count_lo: u16,
    /// Flags
    pub flags: u16,
    /// Exclude bitmap location (lo)
    pub exclude_bitmap_lo: u32,
    /// Block bitmap checksum (lo)
    pub block_bitmap_csum_lo: u16,
    /// Inode bitmap checksum (lo)
    pub inode_bitmap_csum_lo: u16,
    /// Unused inode count
    pub itable_unused_lo: u16,
    /// Checksum
    pub checksum: u16,
}

/// ext4 Inode (on-disk format, 128 bytes minimum)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Inode {
    /// File mode (type and permissions)
    pub mode: u16,
    /// Owner UID
    pub uid: u16,
    /// Size (low 32 bits)
    pub size_lo: u32,
    /// Access time
    pub atime: u32,
    /// Change time
    pub ctime: u32,
    /// Modification time
    pub mtime: u32,
    /// Deletion time
    pub dtime: u32,
    /// Owner GID
    pub gid: u16,
    /// Link count
    pub links_count: u16,
    /// Block count (512-byte units)
    pub blocks_lo: u32,
    /// Flags
    pub flags: u32,
    /// OS-dependent value 1
    pub osd1: u32,
    /// Block pointers or extent tree
    pub block: [u32; 15],
    /// Generation number
    pub generation: u32,
    /// File ACL (low 32 bits)
    pub file_acl_lo: u32,
    /// Size (high 32 bits) / directory ACL
    pub size_high: u32,
    /// Fragment address (obsolete)
    pub obso_faddr: u32,
    /// OS-dependent value 2
    pub osd2: [u8; 12],
    /// Extra inode size
    pub extra_isize: u16,
    /// Checksum (high 16 bits)
    pub checksum_hi: u16,
    /// Change time extra (nanoseconds)
    pub ctime_extra: u32,
    /// Modification time extra
    pub mtime_extra: u32,
    /// Access time extra
    pub atime_extra: u32,
    /// Creation time
    pub crtime: u32,
    /// Creation time extra
    pub crtime_extra: u32,
    /// Version (high 32 bits)
    pub version_hi: u32,
    /// Project ID
    pub projid: u32,
}

/// Inode flags
pub mod InodeFlags {
    pub const EXTENTS: u32 = 0x00080000;
    pub const INLINE_DATA: u32 = 0x10000000;
}

/// File mode constants
pub mod FileMode {
    pub const S_IFMT: u16 = 0o170000;
    pub const S_IFSOCK: u16 = 0o140000;
    pub const S_IFLNK: u16 = 0o120000;
    pub const S_IFREG: u16 = 0o100000;
    pub const S_IFBLK: u16 = 0o060000;
    pub const S_IFDIR: u16 = 0o040000;
    pub const S_IFCHR: u16 = 0o020000;
    pub const S_IFIFO: u16 = 0o010000;
}

impl Inode {
    /// Get file size (64-bit)
    pub fn size(&self) -> u64 {
        ((self.size_high as u64) << 32) | (self.size_lo as u64)
    }
    
    /// Get file type
    pub fn file_type(&self) -> InodeType {
        match self.mode & FileMode::S_IFMT {
            FileMode::S_IFDIR => InodeType::Directory,
            FileMode::S_IFLNK => InodeType::Symlink,
            _ => InodeType::File,
        }
    }
    
    /// Check if uses extents
    pub fn uses_extents(&self) -> bool {
        (self.flags & InodeFlags::EXTENTS) != 0
    }
    
    /// Get permissions (rwxrwxrwx)
    pub fn permissions(&self) -> u16 {
        self.mode & 0o777
    }
}

/// Extent header
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ExtentHeader {
    pub magic: u16,
    pub entries: u16,
    pub max: u16,
    pub depth: u16,
    pub generation: u32,
}

impl ExtentHeader {
    pub const MAGIC: u16 = 0xF30A;
    
    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
    }
}

/// Extent leaf entry
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Extent {
    /// First file block covered
    pub block: u32,
    /// Number of blocks covered
    pub len: u16,
    /// Physical block (high bits)
    pub start_hi: u16,
    /// Physical block (low bits)
    pub start_lo: u32,
}

impl Extent {
    /// Get physical block number (48-bit)
    pub fn start(&self) -> u64 {
        ((self.start_hi as u64) << 32) | (self.start_lo as u64)
    }
}

/// Directory entry
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DirEntryDisk {
    /// Inode number
    pub inode: u32,
    /// Entry length
    pub rec_len: u16,
    /// Name length
    pub name_len: u8,
    /// File type
    pub file_type: u8,
    // Name follows (variable length)
}

/// Directory entry file types
pub mod DirType {
    pub const UNKNOWN: u8 = 0;
    pub const REG_FILE: u8 = 1;
    pub const DIR: u8 = 2;
    pub const CHRDEV: u8 = 3;
    pub const BLKDEV: u8 = 4;
    pub const FIFO: u8 = 5;
    pub const SOCK: u8 = 6;
    pub const SYMLINK: u8 = 7;
}

/// ext4 Filesystem instance
pub struct Ext4Fs {
    /// Superblock
    superblock: Superblock,
    /// Block size
    block_size: u32,
}

impl Ext4Fs {
    /// Mount an ext4 filesystem
    pub fn mount() -> FsResult<Self> {
        // Read superblock
        let mut sb_buf = [0u8; SUPERBLOCK_SIZE];
        block::read_bytes(SUPERBLOCK_OFFSET, &mut sb_buf)
            .map_err(|_| FsError::IoError)?;
        
        // Parse superblock
        let superblock = unsafe {
            core::ptr::read_unaligned(sb_buf.as_ptr() as *const Superblock)
        };
        
        if !superblock.is_valid() {
            return Err(FsError::InvalidFilesystem);
        }
        
        let block_size = superblock.block_size();
        
        Ok(Ext4Fs {
            superblock,
            block_size,
        })
    }
    
    /// Read a block
    fn read_block(&self, block_num: u64, buf: &mut [u8]) -> FsResult<()> {
        let offset = block_num * self.block_size as u64;
        block::read_bytes(offset, buf)
            .map_err(|_| FsError::IoError)
    }
    
    /// Read an inode
    pub fn read_inode(&self, inode_num: u32) -> FsResult<Inode> {
        if inode_num == 0 || inode_num > self.superblock.inodes_count {
            return Err(FsError::NotFound);
        }
        
        // Calculate block group
        let bg = (inode_num - 1) / self.superblock.inodes_per_group;
        let index = (inode_num - 1) % self.superblock.inodes_per_group;
        
        // Read block group descriptor
        let desc_block = if self.block_size == 1024 { 2 } else { 1 };
        let desc_offset = desc_block * self.block_size as u64 + 
                          (bg as u64 * core::mem::size_of::<BlockGroupDesc32>() as u64);
        
        let mut desc_buf = [0u8; 32];
        block::read_bytes(desc_offset, &mut desc_buf)
            .map_err(|_| FsError::IoError)?;
        
        let desc = unsafe {
            core::ptr::read_unaligned(desc_buf.as_ptr() as *const BlockGroupDesc32)
        };
        
        // Calculate inode table offset
        let inode_size = self.superblock.inode_size as u64;
        let inode_offset = (desc.inode_table_lo as u64 * self.block_size as u64) +
                           (index as u64 * inode_size);
        
        let mut inode_buf = [0u8; 256];
        block::read_bytes(inode_offset, &mut inode_buf[..inode_size as usize])
            .map_err(|_| FsError::IoError)?;
        
        Ok(unsafe {
            core::ptr::read_unaligned(inode_buf.as_ptr() as *const Inode)
        })
    }
    
    /// Get data block for a file
    fn get_data_block(&self, inode: &Inode, block_num: u32) -> FsResult<u64> {
        if inode.uses_extents() {
            self.get_extent_block(inode, block_num)
        } else {
            self.get_indirect_block(inode, block_num)
        }
    }
    
    /// Get block from extent tree
    fn get_extent_block(&self, inode: &Inode, logical_block: u32) -> FsResult<u64> {
        // Copy block data to local variable to avoid alignment issues
        let block_copy = inode.block;
        let block_data: [u8; 60] = unsafe {
            core::mem::transmute::<[u32; 15], [u8; 60]>(block_copy)
        };
        
        let header = unsafe {
            core::ptr::read_unaligned(block_data.as_ptr() as *const ExtentHeader)
        };
        
        if !header.is_valid() {
            return Err(FsError::InvalidFilesystem);
        }
        
        // For depth=0, extents follow header directly
        if header.depth == 0 {
            for i in 0..header.entries as usize {
                let offset = 12 + i * 12;
                let extent = unsafe {
                    core::ptr::read_unaligned(
                        block_data.as_ptr().add(offset) as *const Extent
                    )
                };
                
                let len = extent.len as u32;
                if logical_block >= extent.block && logical_block < extent.block + len {
                    let offset = (logical_block - extent.block) as u64;
                    return Ok(extent.start() + offset);
                }
            }
        }
        
        // Block not found
        Err(FsError::NotFound)
    }
    
    /// Get block from indirect block pointers
    fn get_indirect_block(&self, inode: &Inode, block_num: u32) -> FsResult<u64> {
        let ptrs_per_block = self.block_size / 4;
        
        if block_num < 12 {
            // Direct blocks
            Ok(inode.block[block_num as usize] as u64)
        } else if block_num < 12 + ptrs_per_block {
            // Single indirect
            let idx = block_num - 12;
            let indirect_block = inode.block[12] as u64;
            
            let mut buf = vec![0u8; self.block_size as usize];
            self.read_block(indirect_block, &mut buf)?;
            
            let ptr = unsafe {
                core::ptr::read_unaligned(
                    buf.as_ptr().add((idx * 4) as usize) as *const u32
                )
            };
            Ok(ptr as u64)
        } else {
            // Double/triple indirect - simplified
            Err(FsError::NotSupported)
        }
    }
    
    /// Read directory entries
    pub fn read_dir(&self, inode_num: u32) -> FsResult<Vec<DirEntry>> {
        let inode = self.read_inode(inode_num)?;
        
        if inode.file_type() != InodeType::Directory {
            return Err(FsError::NotADirectory);
        }
        
        let mut entries = Vec::new();
        let size = inode.size() as usize;
        let blocks_needed = (size + self.block_size as usize - 1) / self.block_size as usize;
        
        for block_idx in 0..blocks_needed {
            let phys_block = self.get_data_block(&inode, block_idx as u32)?;
            if phys_block == 0 {
                continue;
            }
            
            let mut buf = vec![0u8; self.block_size as usize];
            self.read_block(phys_block, &mut buf)?;
            
            let mut offset = 0;
            while offset < self.block_size as usize {
                if offset + 8 > buf.len() {
                    break;
                }
                
                let entry = unsafe {
                    core::ptr::read_unaligned(
                        buf.as_ptr().add(offset) as *const DirEntryDisk
                    )
                };
                
                if entry.rec_len == 0 {
                    break;
                }
                
                if entry.inode != 0 && entry.name_len > 0 {
                    let name_end = offset + 8 + entry.name_len as usize;
                    if name_end <= buf.len() {
                        let name = String::from_utf8_lossy(
                            &buf[offset + 8..name_end]
                        ).to_string();
                        
                        let inode_type = match entry.file_type {
                            DirType::DIR => InodeType::Directory,
                            DirType::SYMLINK => InodeType::Symlink,
                            _ => InodeType::File,
                        };
                        
                        entries.push(DirEntry {
                            name,
                            inode: entry.inode as u64,
                            inode_type,
                            size: 0, // Size unknown without reading inode
                        });
                    }
                }
                
                offset += entry.rec_len as usize;
            }
        }
        
        Ok(entries)
    }
    
    /// Read file data
    pub fn read_file(&self, inode_num: u32) -> FsResult<Vec<u8>> {
        let inode = self.read_inode(inode_num)?;
        
        if inode.file_type() == InodeType::Directory {
            return Err(FsError::IsADirectory);
        }
        
        let size = inode.size() as usize;
        let mut data = Vec::with_capacity(size);
        
        let blocks_needed = (size + self.block_size as usize - 1) / self.block_size as usize;
        
        for block_idx in 0..blocks_needed {
            let phys_block = self.get_data_block(&inode, block_idx as u32)?;
            
            if phys_block == 0 {
                // Sparse block - fill with zeros
                let to_add = (size - data.len()).min(self.block_size as usize);
                data.extend(core::iter::repeat(0u8).take(to_add));
            } else {
                let mut buf = vec![0u8; self.block_size as usize];
                self.read_block(phys_block, &mut buf)?;
                
                let to_add = (size - data.len()).min(self.block_size as usize);
                data.extend_from_slice(&buf[..to_add]);
            }
        }
        
        Ok(data)
    }
    
    /// Get filesystem info
    pub fn info(&self) -> (u64, u64, u32, String) {
        (
            self.superblock.blocks_count(),
            self.superblock.free_blocks_count_lo as u64 | 
                ((self.superblock.free_blocks_count_hi as u64) << 32),
            self.block_size,
            self.superblock.volume_name(),
        )
    }
    
    /// Lookup path and return inode number
    pub fn lookup(&self, path: &str) -> FsResult<u32> {
        let mut current_inode = 2; // Root inode
        
        for component in path.split('/').filter(|s| !s.is_empty()) {
            let entries = self.read_dir(current_inode)?;
            
            let entry = entries.iter()
                .find(|e| e.name == component)
                .ok_or(FsError::NotFound)?;
            
            current_inode = entry.inode as u32;
        }
        
        Ok(current_inode)
    }
    
    /// Get stat for an inode
    pub fn stat(&self, inode_num: u32) -> FsResult<Stat> {
        let inode = self.read_inode(inode_num)?;
        
        Ok(Stat {
            inode: inode_num as u64,
            inode_type: inode.file_type(),
            size: inode.size() as usize,
            permissions: inode.permissions(),
            uid: inode.uid as u32,
            gid: inode.gid as u32,
        })
    }
}

/// Global ext4 filesystem instance
static EXT4_FS: Mutex<Option<Ext4Fs>> = Mutex::new(None);

/// Mount ext4 filesystem
pub fn mount() -> FsResult<()> {
    let fs = Ext4Fs::mount()?;
    *EXT4_FS.lock() = Some(fs);
    Ok(())
}

/// Check if ext4 is mounted
pub fn is_mounted() -> bool {
    EXT4_FS.lock().is_some()
}

/// List directory
pub fn list_dir(path: &str) -> FsResult<Vec<DirEntry>> {
    let fs = EXT4_FS.lock();
    let fs = fs.as_ref().ok_or(FsError::NoFilesystem)?;
    
    let inode = fs.lookup(path)?;
    fs.read_dir(inode)
}

/// Read file
pub fn read_file(path: &str) -> FsResult<Vec<u8>> {
    let fs = EXT4_FS.lock();
    let fs = fs.as_ref().ok_or(FsError::NoFilesystem)?;
    
    let inode = fs.lookup(path)?;
    fs.read_file(inode)
}

/// Get file stat
pub fn stat(path: &str) -> FsResult<Stat> {
    let fs = EXT4_FS.lock();
    let fs = fs.as_ref().ok_or(FsError::NoFilesystem)?;
    
    let inode = fs.lookup(path)?;
    fs.stat(inode)
}

/// Get filesystem info
pub fn info() -> Option<(u64, u64, u32, String)> {
    EXT4_FS.lock().as_ref().map(|fs| fs.info())
}

