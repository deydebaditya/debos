//! FAT32 Filesystem Driver
//!
//! Implementation of the FAT32 filesystem for reading block devices.
//!
//! ## FAT32 Structure
//! - Boot Sector (sector 0): Contains BPB (BIOS Parameter Block)
//! - Reserved Sectors: Includes backup boot sector
//! - FAT Region: File Allocation Table(s)
//! - Data Region: Clusters containing file/directory data

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::str;

use crate::drivers::block::{self, BlockError};

/// FAT32 Boot Sector / BPB (BIOS Parameter Block)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Fat32BootSector {
    /// Jump instruction to boot code
    pub jump_boot: [u8; 3],
    /// OEM Name
    pub oem_name: [u8; 8],
    /// Bytes per sector (usually 512)
    pub bytes_per_sector: u16,
    /// Sectors per cluster
    pub sectors_per_cluster: u8,
    /// Reserved sector count (including boot sector)
    pub reserved_sectors: u16,
    /// Number of FATs (usually 2)
    pub num_fats: u8,
    /// Root entry count (0 for FAT32)
    pub root_entry_count: u16,
    /// Total sectors (16-bit, 0 for FAT32)
    pub total_sectors_16: u16,
    /// Media type
    pub media_type: u8,
    /// FAT size (16-bit, 0 for FAT32)
    pub fat_size_16: u16,
    /// Sectors per track
    pub sectors_per_track: u16,
    /// Number of heads
    pub num_heads: u16,
    /// Hidden sectors
    pub hidden_sectors: u32,
    /// Total sectors (32-bit)
    pub total_sectors_32: u32,
    // FAT32 specific fields
    /// FAT size (32-bit)
    pub fat_size_32: u32,
    /// Extended flags
    pub ext_flags: u16,
    /// Filesystem version
    pub fs_version: u16,
    /// Root cluster number
    pub root_cluster: u32,
    /// FSInfo sector number
    pub fs_info: u16,
    /// Backup boot sector
    pub backup_boot_sector: u16,
    /// Reserved
    pub reserved: [u8; 12],
    /// Drive number
    pub drive_num: u8,
    /// Reserved
    pub reserved1: u8,
    /// Boot signature (0x29)
    pub boot_sig: u8,
    /// Volume serial number
    pub volume_id: u32,
    /// Volume label
    pub volume_label: [u8; 11],
    /// Filesystem type string
    pub fs_type: [u8; 8],
}

/// FAT32 Directory Entry (32 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Fat32DirEntry {
    /// Short filename (8.3 format)
    pub name: [u8; 11],
    /// Attributes
    pub attr: u8,
    /// Reserved for Windows NT
    pub nt_res: u8,
    /// Creation time (tenths of second)
    pub crt_time_tenth: u8,
    /// Creation time
    pub crt_time: u16,
    /// Creation date
    pub crt_date: u16,
    /// Last access date
    pub lst_acc_date: u16,
    /// High word of first cluster
    pub first_cluster_hi: u16,
    /// Last write time
    pub wrt_time: u16,
    /// Last write date
    pub wrt_date: u16,
    /// Low word of first cluster
    pub first_cluster_lo: u16,
    /// File size
    pub file_size: u32,
}

/// Directory entry attributes
pub mod attr {
    pub const READ_ONLY: u8 = 0x01;
    pub const HIDDEN: u8 = 0x02;
    pub const SYSTEM: u8 = 0x04;
    pub const VOLUME_ID: u8 = 0x08;
    pub const DIRECTORY: u8 = 0x10;
    pub const ARCHIVE: u8 = 0x20;
    pub const LONG_NAME: u8 = 0x0F;
}

/// FAT entry values
pub mod fat_entry {
    pub const FREE: u32 = 0x00000000;
    pub const BAD: u32 = 0x0FFFFFF7;
    pub const END_MIN: u32 = 0x0FFFFFF8;
    pub const END_MAX: u32 = 0x0FFFFFFF;
}

/// FAT32 filesystem instance
pub struct Fat32 {
    /// Bytes per sector
    pub bytes_per_sector: u32,
    /// Sectors per cluster
    pub sectors_per_cluster: u32,
    /// First FAT sector
    pub fat_start_sector: u32,
    /// First data sector
    pub data_start_sector: u32,
    /// Root directory cluster
    pub root_cluster: u32,
    /// FAT size in sectors
    pub fat_size: u32,
    /// Total clusters
    pub total_clusters: u32,
    /// Volume label
    pub volume_label: String,
}

/// FAT32 error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fat32Error {
    /// Not a FAT32 filesystem
    InvalidFilesystem,
    /// Block device error
    BlockError(BlockError),
    /// File not found
    NotFound,
    /// Not a directory
    NotADirectory,
    /// Not a file
    NotAFile,
    /// Invalid path
    InvalidPath,
    /// End of file
    EndOfFile,
}

impl From<BlockError> for Fat32Error {
    fn from(e: BlockError) -> Self {
        Fat32Error::BlockError(e)
    }
}

impl Fat32 {
    /// Mount a FAT32 filesystem from the block device
    pub fn mount() -> Result<Self, Fat32Error> {
        // Read boot sector
        let mut buf = [0u8; 512];
        block::read_bytes(0, &mut buf)?;
        
        // Parse boot sector
        let boot = unsafe { *(buf.as_ptr() as *const Fat32BootSector) };
        
        // Validate FAT32 signature
        if boot.boot_sig != 0x29 {
            return Err(Fat32Error::InvalidFilesystem);
        }
        
        // Check for FAT32 (FAT size 16 should be 0)
        if boot.fat_size_16 != 0 {
            return Err(Fat32Error::InvalidFilesystem);
        }
        
        let bytes_per_sector = boot.bytes_per_sector as u32;
        let sectors_per_cluster = boot.sectors_per_cluster as u32;
        let reserved_sectors = boot.reserved_sectors as u32;
        let num_fats = boot.num_fats as u32;
        let fat_size = boot.fat_size_32;
        let root_cluster = boot.root_cluster;
        
        // Calculate sector positions
        let fat_start_sector = reserved_sectors;
        let data_start_sector = reserved_sectors + (num_fats * fat_size);
        
        // Calculate total clusters
        let total_sectors = boot.total_sectors_32;
        let data_sectors = total_sectors - data_start_sector;
        let total_clusters = data_sectors / sectors_per_cluster;
        
        // Parse volume label
        let volume_label = str::from_utf8(&boot.volume_label)
            .unwrap_or("NO NAME")
            .trim()
            .to_string();
        
        Ok(Fat32 {
            bytes_per_sector,
            sectors_per_cluster,
            fat_start_sector,
            data_start_sector,
            root_cluster,
            fat_size,
            total_clusters,
            volume_label,
        })
    }
    
    /// Convert cluster number to sector number
    fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.data_start_sector + (cluster - 2) * self.sectors_per_cluster
    }
    
    /// Read a FAT entry
    fn read_fat_entry(&self, cluster: u32) -> Result<u32, Fat32Error> {
        let fat_offset = cluster * 4;
        let fat_sector = self.fat_start_sector + (fat_offset / self.bytes_per_sector);
        let entry_offset = (fat_offset % self.bytes_per_sector) as usize;
        
        let mut buf = [0u8; 512];
        let sector_byte_offset = fat_sector as u64 * self.bytes_per_sector as u64;
        block::read_bytes(sector_byte_offset, &mut buf)?;
        
        let entry = u32::from_le_bytes([
            buf[entry_offset],
            buf[entry_offset + 1],
            buf[entry_offset + 2],
            buf[entry_offset + 3],
        ]);
        
        Ok(entry & 0x0FFFFFFF)
    }
    
    /// Read a cluster into a buffer
    fn read_cluster(&self, cluster: u32, buf: &mut [u8]) -> Result<(), Fat32Error> {
        let sector = self.cluster_to_sector(cluster);
        let cluster_size = (self.sectors_per_cluster * self.bytes_per_sector) as usize;
        
        if buf.len() < cluster_size {
            return Err(Fat32Error::InvalidPath);
        }
        
        let byte_offset = sector as u64 * self.bytes_per_sector as u64;
        block::read_bytes(byte_offset, &mut buf[..cluster_size])?;
        
        Ok(())
    }
    
    /// Get cluster chain for a starting cluster
    fn get_cluster_chain(&self, start_cluster: u32) -> Result<Vec<u32>, Fat32Error> {
        let mut chain = Vec::new();
        let mut current = start_cluster;
        
        while current >= 2 && current < fat_entry::END_MIN {
            chain.push(current);
            current = self.read_fat_entry(current)?;
        }
        
        Ok(chain)
    }
    
    /// List directory entries in a cluster
    pub fn list_directory(&self, dir_cluster: u32) -> Result<Vec<(String, Fat32DirEntry)>, Fat32Error> {
        let cluster_size = (self.sectors_per_cluster * self.bytes_per_sector) as usize;
        let entries_per_cluster = cluster_size / 32;
        let mut buf = alloc::vec![0u8; cluster_size];
        let mut result = Vec::new();
        
        let clusters = self.get_cluster_chain(dir_cluster)?;
        
        for cluster in clusters {
            self.read_cluster(cluster, &mut buf)?;
            
            for i in 0..entries_per_cluster {
                let entry = unsafe {
                    *(buf.as_ptr().add(i * 32) as *const Fat32DirEntry)
                };
                
                // Check for end of directory
                if entry.name[0] == 0x00 {
                    return Ok(result);
                }
                
                // Skip deleted entries
                if entry.name[0] == 0xE5 {
                    continue;
                }
                
                // Skip long filename entries (we only support 8.3 names)
                if entry.attr == attr::LONG_NAME {
                    continue;
                }
                
                // Skip volume ID
                if entry.attr & attr::VOLUME_ID != 0 {
                    continue;
                }
                
                // Parse 8.3 filename
                let name = self.parse_short_name(&entry.name);
                result.push((name, entry));
            }
        }
        
        Ok(result)
    }
    
    /// Parse 8.3 short filename
    fn parse_short_name(&self, raw: &[u8; 11]) -> String {
        let mut name = String::new();
        
        // Name part (first 8 bytes)
        for &c in &raw[0..8] {
            if c == 0x20 {
                break;
            }
            name.push(c as char);
        }
        
        // Extension part (last 3 bytes)
        if raw[8] != 0x20 {
            name.push('.');
            for &c in &raw[8..11] {
                if c == 0x20 {
                    break;
                }
                name.push(c as char);
            }
        }
        
        name.to_lowercase()
    }
    
    /// Find a file/directory by path
    pub fn find_entry(&self, path: &str) -> Result<Fat32DirEntry, Fat32Error> {
        let mut current_cluster = self.root_cluster;
        
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            // Return a fake root entry
            return Ok(Fat32DirEntry {
                name: *b"/          ",
                attr: attr::DIRECTORY,
                nt_res: 0,
                crt_time_tenth: 0,
                crt_time: 0,
                crt_date: 0,
                lst_acc_date: 0,
                first_cluster_hi: (self.root_cluster >> 16) as u16,
                wrt_time: 0,
                wrt_date: 0,
                first_cluster_lo: self.root_cluster as u16,
                file_size: 0,
            });
        }
        
        for component in path.split('/') {
            if component.is_empty() {
                continue;
            }
            
            let entries = self.list_directory(current_cluster)?;
            let component_upper = component.to_uppercase();
            
            let mut found = false;
            for (name, entry) in entries {
                if name.to_uppercase() == component_upper {
                    if entry.attr & attr::DIRECTORY != 0 {
                        current_cluster = ((entry.first_cluster_hi as u32) << 16) 
                            | entry.first_cluster_lo as u32;
                    }
                    
                    // Check if this is the last component
                    if path.ends_with(component) {
                        return Ok(entry);
                    }
                    
                    found = true;
                    break;
                }
            }
            
            if !found {
                return Err(Fat32Error::NotFound);
            }
        }
        
        Err(Fat32Error::NotFound)
    }
    
    /// Read file contents
    pub fn read_file(&self, entry: &Fat32DirEntry) -> Result<Vec<u8>, Fat32Error> {
        if entry.attr & attr::DIRECTORY != 0 {
            return Err(Fat32Error::NotAFile);
        }
        
        let start_cluster = ((entry.first_cluster_hi as u32) << 16) 
            | entry.first_cluster_lo as u32;
        let file_size = entry.file_size as usize;
        let cluster_size = (self.sectors_per_cluster * self.bytes_per_sector) as usize;
        
        let mut data = Vec::with_capacity(file_size);
        let mut buf = alloc::vec![0u8; cluster_size];
        let clusters = self.get_cluster_chain(start_cluster)?;
        let mut remaining = file_size;
        
        for cluster in clusters {
            self.read_cluster(cluster, &mut buf)?;
            
            let to_copy = remaining.min(cluster_size);
            data.extend_from_slice(&buf[..to_copy]);
            remaining -= to_copy;
            
            if remaining == 0 {
                break;
            }
        }
        
        Ok(data)
    }
    
    /// List root directory
    pub fn list_root(&self) -> Result<Vec<(String, Fat32DirEntry)>, Fat32Error> {
        self.list_directory(self.root_cluster)
    }
}

/// Global FAT32 instance
static mut FAT32_FS: Option<Fat32> = None;

/// Mount FAT32 filesystem
pub fn mount() -> Result<(), Fat32Error> {
    let fs = Fat32::mount()?;
    crate::println!("  FAT32 mounted: \"{}\" ({} clusters)", 
        fs.volume_label, fs.total_clusters);
    
    unsafe {
        FAT32_FS = Some(fs);
    }
    Ok(())
}

/// Get the mounted FAT32 filesystem
pub fn get_fs() -> Option<&'static Fat32> {
    unsafe { FAT32_FS.as_ref() }
}

/// Check if FAT32 is mounted
pub fn is_mounted() -> bool {
    unsafe { FAT32_FS.is_some() }
}

/// List directory at path
pub fn ls(path: &str) -> Result<Vec<(String, bool, u32)>, Fat32Error> {
    let fs = get_fs().ok_or(Fat32Error::InvalidFilesystem)?;
    
    let cluster = if path == "/" || path.is_empty() {
        fs.root_cluster
    } else {
        let entry = fs.find_entry(path)?;
        if entry.attr & attr::DIRECTORY == 0 {
            return Err(Fat32Error::NotADirectory);
        }
        ((entry.first_cluster_hi as u32) << 16) | entry.first_cluster_lo as u32
    };
    
    let entries = fs.list_directory(cluster)?;
    Ok(entries.into_iter()
        .map(|(name, entry)| {
            let is_dir = entry.attr & attr::DIRECTORY != 0;
            let size = entry.file_size;
            (name, is_dir, size)
        })
        .collect())
}

/// Read file contents
pub fn read_file(path: &str) -> Result<Vec<u8>, Fat32Error> {
    let fs = get_fs().ok_or(Fat32Error::InvalidFilesystem)?;
    let entry = fs.find_entry(path)?;
    fs.read_file(&entry)
}

