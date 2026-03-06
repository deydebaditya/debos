//! RAM Filesystem Implementation
//!
//! A simple in-memory filesystem using inodes and directory entries.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{InodeType, FsError, FsResult, Stat, DirEntry};
use super::path;

/// Maximum file size (1MB for now)
const MAX_FILE_SIZE: usize = 1024 * 1024;

/// An inode representing a file or directory
#[derive(Debug)]
pub struct Inode {
    /// Unique inode ID
    pub id: u64,
    /// Type of inode (file, directory, symlink)
    pub inode_type: InodeType,
    /// Name of this entry
    pub name: String,
    /// Size in bytes (for files)
    pub size: usize,
    /// Unix-style permissions (e.g., 0o755)
    pub permissions: u16,
    /// Owner user ID
    pub uid: u32,
    /// Owner group ID
    pub gid: u32,
    /// File data (for files)
    pub data: Vec<u8>,
    /// Child inode IDs (for directories)
    pub children: Vec<u64>,
    /// Parent inode ID (None for root)
    pub parent: Option<u64>,
}

impl Inode {
    /// Create a new file inode with specified owner
    pub fn new_file(id: u64, name: String, parent: u64, uid: u32, gid: u32) -> Self {
        Inode {
            id,
            inode_type: InodeType::File,
            name,
            size: 0,
            permissions: 0o644,
            uid,
            gid,
            data: Vec::new(),
            children: Vec::new(),
            parent: Some(parent),
        }
    }
    
    /// Create a new directory inode with specified owner
    pub fn new_directory(id: u64, name: String, parent: Option<u64>, uid: u32, gid: u32) -> Self {
        Inode {
            id,
            inode_type: InodeType::Directory,
            name,
            size: 0,
            permissions: 0o755,
            uid,
            gid,
            data: Vec::new(),
            children: Vec::new(),
            parent,
        }
    }
    
    /// Create a new file inode with specific owner
    pub fn new_file_with_owner(id: u64, name: String, parent: u64, uid: u32, gid: u32) -> Self {
        Inode {
            id,
            inode_type: InodeType::File,
            name,
            size: 0,
            permissions: 0o644,
            uid,
            gid,
            data: Vec::new(),
            children: Vec::new(),
            parent: Some(parent),
        }
    }
    
    /// Get metadata as Stat
    pub fn stat(&self) -> Stat {
        Stat {
            inode: self.id,
            inode_type: self.inode_type,
            size: self.size,
            permissions: self.permissions,
            uid: self.uid,
            gid: self.gid,
        }
    }
}

/// Get current user's uid/gid for file ownership
fn get_current_owner() -> (u32, u32) {
    // Try to get from current thread's credentials
    if let Some(creds) = crate::scheduler::current_credentials() {
        (creds.uid.as_raw(), creds.gid.as_raw())
    } else {
        // Kernel context - root
        (0, 0)
    }
}

/// RAM-based filesystem
pub struct RamFs {
    /// All inodes indexed by ID
    inodes: BTreeMap<u64, Inode>,
    /// Next available inode ID
    next_inode_id: AtomicU64,
    /// Root inode ID
    root_inode: u64,
}

impl RamFs {
    /// Create a new RamFS with a root directory
    pub fn new() -> Self {
        let mut inodes = BTreeMap::new();
        
        // Create root directory (inode 1)
        let root = Inode::new_directory(1, String::from("/"), None, 0, 0); // Root directory owned by root
        inodes.insert(1, root);
        
        RamFs {
            inodes,
            next_inode_id: AtomicU64::new(2),
            root_inode: 1,
        }
    }
    
    /// Get the root inode ID
    pub fn root(&self) -> u64 {
        self.root_inode
    }
    
    /// Allocate a new inode ID
    fn alloc_inode_id(&self) -> u64 {
        self.next_inode_id.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Look up an inode by path, returning its ID
    pub fn lookup(&self, path: &str) -> FsResult<u64> {
        let normalized = path::normalize_path(path);
        
        if normalized == "/" {
            return Ok(self.root_inode);
        }
        
        let components = path::components(&normalized);
        let mut current = self.root_inode;
        
        for component in components {
            let inode = self.inodes.get(&current).ok_or(FsError::NotFound)?;
            
            if inode.inode_type != InodeType::Directory {
                return Err(FsError::NotADirectory);
            }
            
            // Find child with matching name
            let mut found = false;
            for &child_id in &inode.children {
                if let Some(child) = self.inodes.get(&child_id) {
                    if child.name == component {
                        current = child_id;
                        found = true;
                        break;
                    }
                }
            }
            
            if !found {
                return Err(FsError::NotFound);
            }
        }
        
        Ok(current)
    }
    
    /// Get an inode by ID
    pub fn get(&self, id: u64) -> FsResult<&Inode> {
        self.inodes.get(&id).ok_or(FsError::NotFound)
    }
    
    /// Get a mutable inode by ID
    pub fn get_mut(&mut self, id: u64) -> FsResult<&mut Inode> {
        self.inodes.get_mut(&id).ok_or(FsError::NotFound)
    }
    
    /// Create a file with specified owner
    pub fn create_file(&mut self, dir_path: &str, name: &str, uid: u32, gid: u32) -> FsResult<u64> {
        self.create_inode(dir_path, name, InodeType::File, uid, gid)
    }
    
    /// Create a directory with specified owner
    pub fn create_dir(&mut self, dir_path: &str, name: &str, uid: u32, gid: u32) -> FsResult<u64> {
        self.create_inode(dir_path, name, InodeType::Directory, uid, gid)
    }
    
    /// Create an inode of the specified type with specified owner
    fn create_inode(&mut self, dir_path: &str, name: &str, inode_type: InodeType, uid: u32, gid: u32) -> FsResult<u64> {
        // Validate name
        if name.is_empty() || name.contains('/') {
            return Err(FsError::InvalidPath);
        }
        
        // Find parent directory
        let parent_id = self.lookup(dir_path)?;
        
        {
            let parent = self.inodes.get(&parent_id).ok_or(FsError::NotFound)?;
            
            if parent.inode_type != InodeType::Directory {
                return Err(FsError::NotADirectory);
            }
            
            // Check if name already exists
            for &child_id in &parent.children {
                if let Some(child) = self.inodes.get(&child_id) {
                    if child.name == name {
                        return Err(FsError::AlreadyExists);
                    }
                }
            }
        }
        
        // Create new inode
        let new_id = self.alloc_inode_id();
        let new_inode = match inode_type {
            InodeType::File => Inode::new_file(new_id, String::from(name), parent_id, uid, gid),
            InodeType::Directory => Inode::new_directory(new_id, String::from(name), Some(parent_id), uid, gid),
            InodeType::Symlink => return Err(FsError::InvalidArgument), // Not implemented
        };
        
        self.inodes.insert(new_id, new_inode);
        
        // Add to parent's children
        if let Some(parent) = self.inodes.get_mut(&parent_id) {
            parent.children.push(new_id);
        }
        
        Ok(new_id)
    }
    
    /// Remove a file or empty directory
    pub fn remove(&mut self, path: &str) -> FsResult<()> {
        let normalized = path::normalize_path(path);
        
        if normalized == "/" {
            return Err(FsError::PermissionDenied);
        }
        
        let inode_id = self.lookup(&normalized)?;
        
        // Check if it's a non-empty directory
        {
            let inode = self.inodes.get(&inode_id).ok_or(FsError::NotFound)?;
            if inode.inode_type == InodeType::Directory && !inode.children.is_empty() {
                return Err(FsError::NotEmpty);
            }
        }
        
        // Get parent ID and remove from parent's children
        let parent_id = {
            let inode = self.inodes.get(&inode_id).ok_or(FsError::NotFound)?;
            inode.parent.ok_or(FsError::PermissionDenied)?
        };
        
        if let Some(parent) = self.inodes.get_mut(&parent_id) {
            parent.children.retain(|&id| id != inode_id);
        }
        
        // Remove the inode
        self.inodes.remove(&inode_id);
        
        Ok(())
    }
    
    /// Remove a file (fails if directory)
    pub fn unlink(&mut self, path: &str) -> FsResult<()> {
        let inode_id = self.lookup(path)?;
        let inode = self.inodes.get(&inode_id).ok_or(FsError::NotFound)?;
        
        if inode.inode_type == InodeType::Directory {
            return Err(FsError::IsADirectory);
        }
        
        self.remove(path)
    }
    
    /// Remove an empty directory
    pub fn rmdir(&mut self, path: &str) -> FsResult<()> {
        let inode_id = self.lookup(path)?;
        let inode = self.inodes.get(&inode_id).ok_or(FsError::NotFound)?;
        
        if inode.inode_type != InodeType::Directory {
            return Err(FsError::NotADirectory);
        }
        
        if !inode.children.is_empty() {
            return Err(FsError::NotEmpty);
        }
        
        self.remove(path)
    }
    
    /// Read file contents
    pub fn read(&self, inode_id: u64, offset: usize, buf: &mut [u8]) -> FsResult<usize> {
        let inode = self.inodes.get(&inode_id).ok_or(FsError::NotFound)?;
        
        if inode.inode_type == InodeType::Directory {
            return Err(FsError::IsADirectory);
        }
        
        if offset >= inode.size {
            return Ok(0);
        }
        
        let available = inode.size - offset;
        let to_read = core::cmp::min(available, buf.len());
        
        buf[..to_read].copy_from_slice(&inode.data[offset..offset + to_read]);
        
        Ok(to_read)
    }
    
    /// Write to file
    pub fn write(&mut self, inode_id: u64, offset: usize, data: &[u8]) -> FsResult<usize> {
        let inode = self.inodes.get_mut(&inode_id).ok_or(FsError::NotFound)?;
        
        if inode.inode_type == InodeType::Directory {
            return Err(FsError::IsADirectory);
        }
        
        let end_pos = offset + data.len();
        
        if end_pos > MAX_FILE_SIZE {
            return Err(FsError::NoSpace);
        }
        
        // Extend file if necessary
        if end_pos > inode.data.len() {
            inode.data.resize(end_pos, 0);
        }
        
        inode.data[offset..end_pos].copy_from_slice(data);
        inode.size = core::cmp::max(inode.size, end_pos);
        
        Ok(data.len())
    }
    
    /// Truncate file to specified size
    pub fn truncate(&mut self, inode_id: u64, size: usize) -> FsResult<()> {
        let inode = self.inodes.get_mut(&inode_id).ok_or(FsError::NotFound)?;
        
        if inode.inode_type == InodeType::Directory {
            return Err(FsError::IsADirectory);
        }
        
        if size > MAX_FILE_SIZE {
            return Err(FsError::NoSpace);
        }
        
        inode.data.resize(size, 0);
        inode.size = size;
        
        Ok(())
    }
    
    /// Read directory entries
    pub fn readdir(&self, inode_id: u64) -> FsResult<Vec<DirEntry>> {
        let inode = self.inodes.get(&inode_id).ok_or(FsError::NotFound)?;
        
        if inode.inode_type != InodeType::Directory {
            return Err(FsError::NotADirectory);
        }
        
        let mut entries = Vec::new();
        
        for &child_id in &inode.children {
            if let Some(child) = self.inodes.get(&child_id) {
                entries.push(DirEntry {
                    name: child.name.clone(),
                    inode: child.id,
                    inode_type: child.inode_type,
                    size: child.size,
                });
            }
        }
        
        // Sort by name
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        
        Ok(entries)
    }
    
    /// Get inode metadata
    pub fn stat(&self, path: &str) -> FsResult<Stat> {
        let inode_id = self.lookup(path)?;
        let inode = self.inodes.get(&inode_id).ok_or(FsError::NotFound)?;
        Ok(inode.stat())
    }
}

impl Default for RamFs {
    fn default() -> Self {
        Self::new()
    }
}

