//! VFS Server - Virtual Filesystem Server
//!
//! The VFS server is a userspace process that handles all filesystem operations
//! for DebOS. It receives requests via IPC, processes them against mounted
//! filesystems, and returns results.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        User Applications                         │
//! │                    (libdebos filesystem API)                     │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │ IPC (VFS Protocol)
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         VFS Server                               │
//! │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐            │
//! │  │  Mount  │  │  Inode  │  │  Dentry │  │   FD    │            │
//! │  │  Table  │  │  Cache  │  │  Cache  │  │  Table  │            │
//! │  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘            │
//! │       │            │            │            │                   │
//! │  ┌────┴────────────┴────────────┴────────────┴────┐             │
//! │  │            Filesystem Drivers                    │             │
//! │  │  ┌───────┐  ┌───────┐  ┌───────┐  ┌───────┐    │             │
//! │  │  │ RamFS │  │ FAT32 │  │  ext4 │  │ DebFS │    │             │
//! │  │  └───────┘  └───────┘  └───────┘  └───────┘    │             │
//! │  └────────────────────────────────────────────────┘             │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │ Block I/O (via DevMan)
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                       Block Devices                              │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Supported Operations
//!
//! - **File Operations**: open, close, read, write, seek, truncate
//! - **Directory Operations**: mkdir, rmdir, readdir, chdir, getcwd
//! - **Metadata Operations**: stat, chmod, chown
//! - **Path Operations**: unlink, rename, symlink, readlink
//! - **Mount Operations**: mount, unmount
//!

#![no_std]
#![no_main]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// VFS Protocol types (shared with kernel)
mod protocol;
use protocol::*;

/// Maximum open files per process
const MAX_FDS_PER_PROCESS: usize = 256;

/// Maximum total open files
const MAX_TOTAL_FDS: usize = 65536;

/// VFS Server state
struct VfsServer {
    /// Mount points (path -> filesystem)
    mounts: BTreeMap<String, MountPoint>,
    /// Open file descriptors (global fd -> OpenFile)
    open_files: BTreeMap<u64, OpenFile>,
    /// Per-process file descriptor tables (pid -> (local_fd -> global_fd))
    process_fds: BTreeMap<u32, BTreeMap<i32, u64>>,
    /// Per-process current working directory
    process_cwd: BTreeMap<u32, String>,
    /// Next global file descriptor
    next_fd: AtomicU64,
    /// Inode cache
    inode_cache: BTreeMap<(u64, u64), CachedInode>, // (mount_id, inode) -> cache
    /// Dentry cache
    dentry_cache: BTreeMap<String, DentryCacheEntry>,
}

/// A mounted filesystem
struct MountPoint {
    /// Unique mount ID
    id: u64,
    /// Mount path
    path: String,
    /// Filesystem type
    fs_type: FilesystemType,
    /// Filesystem-specific data
    fs_data: FilesystemData,
    /// Read-only flag
    read_only: bool,
}

/// Filesystem types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilesystemType {
    RamFS,
    FAT32,
    Ext4,
    DebFS,
}

/// Filesystem-specific data
enum FilesystemData {
    RamFS(RamFsData),
    FAT32(Fat32Data),
    Ext4(Ext4Data),
    DebFS(DebFsData),
}

/// RamFS filesystem data
struct RamFsData {
    inodes: BTreeMap<u64, RamFsInode>,
    next_inode: u64,
}

/// RamFS inode
struct RamFsInode {
    inode: u64,
    file_type: VfsFileType,
    permissions: u16,
    uid: u32,
    gid: u32,
    size: u64,
    data: Vec<u8>,
    children: BTreeMap<String, u64>, // For directories
    atime: u64,
    mtime: u64,
    ctime: u64,
}

/// FAT32 filesystem data
struct Fat32Data {
    // FAT32 specific structures
    boot_sector: [u8; 512],
    sectors_per_cluster: u8,
    root_cluster: u32,
}

/// ext4 filesystem data  
struct Ext4Data {
    // ext4 specific structures
    superblock: [u8; 1024],
    block_size: u32,
}

/// DebFS filesystem data
struct DebFsData {
    // Future native filesystem
}

/// An open file
struct OpenFile {
    /// Global file descriptor
    fd: u64,
    /// Mount ID
    mount_id: u64,
    /// Inode number
    inode: u64,
    /// Current position
    position: u64,
    /// Open flags
    flags: u32,
    /// Owning process
    pid: u32,
}

/// Cached inode data
struct CachedInode {
    file_type: VfsFileType,
    permissions: u16,
    uid: u32,
    gid: u32,
    size: u64,
    dirty: bool,
}

/// Dentry cache entry
struct DentryCacheEntry {
    mount_id: u64,
    inode: u64,
    valid: bool,
}

impl VfsServer {
    /// Create a new VFS server
    fn new() -> Self {
        Self {
            mounts: BTreeMap::new(),
            open_files: BTreeMap::new(),
            process_fds: BTreeMap::new(),
            process_cwd: BTreeMap::new(),
            next_fd: AtomicU64::new(3), // 0, 1, 2 reserved for stdin/out/err
            inode_cache: BTreeMap::new(),
            dentry_cache: BTreeMap::new(),
        }
    }

    /// Initialize with root RamFS
    fn init_root_ramfs(&mut self) {
        let mut root_inode = RamFsInode {
            inode: 1,
            file_type: VfsFileType::Directory,
            permissions: 0o755,
            uid: 0,
            gid: 0,
            size: 0,
            data: Vec::new(),
            children: BTreeMap::new(),
            atime: 0,
            mtime: 0,
            ctime: 0,
        };
        
        // Add . and .. entries
        root_inode.children.insert(".".to_string(), 1);
        root_inode.children.insert("..".to_string(), 1);
        
        let mut inodes = BTreeMap::new();
        inodes.insert(1, root_inode);
        
        let ramfs_data = RamFsData {
            inodes,
            next_inode: 2,
        };
        
        let mount = MountPoint {
            id: 1,
            path: "/".to_string(),
            fs_type: FilesystemType::RamFS,
            fs_data: FilesystemData::RamFS(ramfs_data),
            read_only: false,
        };
        
        self.mounts.insert("/".to_string(), mount);
    }

    /// Handle an incoming VFS request
    fn handle_request(&mut self, msg: &[u8]) -> Vec<u8> {
        // Parse request header
        let header = match VfsRequestHeader::from_bytes(msg) {
            Some(h) => h,
            None => return self.error_response(0, VfsError::InvalidArgument),
        };
        
        let op = VfsOp::from(header.op);
        
        match op {
            VfsOp::Open => self.handle_open(&header, msg),
            VfsOp::Close => self.handle_close(&header),
            VfsOp::Read => self.handle_read(&header),
            VfsOp::Write => self.handle_write(&header, msg),
            VfsOp::Stat => self.handle_stat(&header, msg),
            VfsOp::Mkdir => self.handle_mkdir(&header, msg),
            VfsOp::Rmdir => self.handle_rmdir(&header, msg),
            VfsOp::Unlink => self.handle_unlink(&header, msg),
            VfsOp::Readdir => self.handle_readdir(&header),
            VfsOp::Seek => self.handle_seek(&header),
            VfsOp::Sync => self.handle_sync(&header),
            VfsOp::Rename => self.handle_rename(&header, msg),
            VfsOp::Truncate => self.handle_truncate(&header),
            VfsOp::Chmod => self.handle_chmod(&header, msg),
            VfsOp::Chown => self.handle_chown(&header, msg),
            VfsOp::Touch => self.handle_touch(&header, msg),
            VfsOp::Chdir => self.handle_chdir(&header, msg),
            VfsOp::Getcwd => self.handle_getcwd(&header),
            VfsOp::Mount => self.handle_mount(&header, msg),
            VfsOp::Unmount => self.handle_unmount(&header, msg),
            _ => self.error_response(header.request_id, VfsError::NotSupported),
        }
    }

    /// Handle OPEN operation
    fn handle_open(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        let path = match parse_path(msg, header) {
            Some(p) => p,
            None => return self.error_response(header.request_id, VfsError::InvalidPath),
        };
        
        // Resolve path to mount point and inode
        let (mount_id, inode) = match self.resolve_path(&path) {
            Ok(r) => r,
            Err(e) => {
                // If CREATE flag and file doesn't exist, create it
                if header.flags & VfsOpenFlags::Create as u32 != 0 && e == VfsError::NotFound {
                    match self.create_file(&path, header.uid, header.gid, 0o644) {
                        Ok((mid, ino)) => (mid, ino),
                        Err(e) => return self.error_response(header.request_id, e),
                    }
                } else {
                    return self.error_response(header.request_id, e);
                }
            }
        };
        
        // Check permissions
        if let Err(e) = self.check_access(mount_id, inode, header.uid, header.gid, header.flags) {
            return self.error_response(header.request_id, e);
        }
        
        // Allocate file descriptor
        let fd = self.next_fd.fetch_add(1, Ordering::Relaxed);
        
        let open_file = OpenFile {
            fd,
            mount_id,
            inode,
            position: 0,
            flags: header.flags,
            pid: header.uid, // Using uid as pid placeholder
        };
        
        self.open_files.insert(fd, open_file);
        
        // Return success with fd
        self.success_response(header.request_id, fd as i32)
    }

    /// Handle CLOSE operation
    fn handle_close(&mut self, header: &VfsRequestHeader) -> Vec<u8> {
        let fd = header.fd as u64;
        
        match self.open_files.remove(&fd) {
            Some(_) => self.success_response(header.request_id, 0),
            None => self.error_response(header.request_id, VfsError::InvalidFd),
        }
    }

    /// Handle READ operation
    fn handle_read(&mut self, header: &VfsRequestHeader) -> Vec<u8> {
        let fd = header.fd as u64;
        
        let (mount_id, inode, position) = match self.open_files.get(&fd) {
            Some(f) => (f.mount_id, f.inode, f.position),
            None => return self.error_response(header.request_id, VfsError::InvalidFd),
        };
        
        // Read data from filesystem
        let data = match self.read_inode_data(mount_id, inode, position, header.length as usize) {
            Ok(d) => d,
            Err(e) => return self.error_response(header.request_id, e),
        };
        
        let bytes_read = data.len();
        
        // Update position
        if let Some(f) = self.open_files.get_mut(&fd) {
            f.position += bytes_read as u64;
        }
        
        // Return data
        self.data_response(header.request_id, bytes_read as i32, &data)
    }

    /// Handle WRITE operation
    fn handle_write(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        let fd = header.fd as u64;
        
        let (mount_id, inode, position, flags) = match self.open_files.get(&fd) {
            Some(f) => (f.mount_id, f.inode, f.position, f.flags),
            None => return self.error_response(header.request_id, VfsError::InvalidFd),
        };
        
        // Check write permission
        if flags & VfsOpenFlags::Write as u32 == 0 {
            return self.error_response(header.request_id, VfsError::PermissionDenied);
        }
        
        // Get data to write
        let data = match parse_data(msg, header) {
            Some(d) => d,
            None => return self.error_response(header.request_id, VfsError::InvalidArgument),
        };
        
        // Determine write position
        let write_pos = if flags & VfsOpenFlags::Append as u32 != 0 {
            // Get file size for append
            match self.get_inode_size(mount_id, inode) {
                Ok(size) => size,
                Err(e) => return self.error_response(header.request_id, e),
            }
        } else {
            position
        };
        
        // Write data to filesystem
        let bytes_written = match self.write_inode_data(mount_id, inode, write_pos, data) {
            Ok(n) => n,
            Err(e) => return self.error_response(header.request_id, e),
        };
        
        // Update position
        if let Some(f) = self.open_files.get_mut(&fd) {
            f.position = write_pos + bytes_written as u64;
        }
        
        self.success_response(header.request_id, bytes_written as i32)
    }

    /// Handle STAT operation
    fn handle_stat(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        let path = match parse_path(msg, header) {
            Some(p) => p,
            None => return self.error_response(header.request_id, VfsError::InvalidPath),
        };
        
        let (mount_id, inode) = match self.resolve_path(&path) {
            Ok(r) => r,
            Err(e) => return self.error_response(header.request_id, e),
        };
        
        let stat = match self.get_stat(mount_id, inode) {
            Ok(s) => s,
            Err(e) => return self.error_response(header.request_id, e),
        };
        
        self.stat_response(header.request_id, &stat)
    }

    /// Handle MKDIR operation
    fn handle_mkdir(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        let path = match parse_path(msg, header) {
            Some(p) => p,
            None => return self.error_response(header.request_id, VfsError::InvalidPath),
        };
        
        match self.create_directory(&path, header.uid, header.gid, header.flags as u16) {
            Ok(_) => self.success_response(header.request_id, 0),
            Err(e) => self.error_response(header.request_id, e),
        }
    }

    /// Handle RMDIR operation
    fn handle_rmdir(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        let path = match parse_path(msg, header) {
            Some(p) => p,
            None => return self.error_response(header.request_id, VfsError::InvalidPath),
        };
        
        match self.remove_directory(&path) {
            Ok(_) => self.success_response(header.request_id, 0),
            Err(e) => self.error_response(header.request_id, e),
        }
    }

    /// Handle UNLINK operation
    fn handle_unlink(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        let path = match parse_path(msg, header) {
            Some(p) => p,
            None => return self.error_response(header.request_id, VfsError::InvalidPath),
        };
        
        match self.remove_file(&path) {
            Ok(_) => self.success_response(header.request_id, 0),
            Err(e) => self.error_response(header.request_id, e),
        }
    }

    /// Handle READDIR operation
    fn handle_readdir(&mut self, header: &VfsRequestHeader) -> Vec<u8> {
        let fd = header.fd as u64;
        
        let (mount_id, inode) = match self.open_files.get(&fd) {
            Some(f) => (f.mount_id, f.inode),
            None => return self.error_response(header.request_id, VfsError::InvalidFd),
        };
        
        match self.list_directory(mount_id, inode) {
            Ok(entries) => self.dirlist_response(header.request_id, &entries),
            Err(e) => self.error_response(header.request_id, e),
        }
    }

    /// Handle SEEK operation
    fn handle_seek(&mut self, header: &VfsRequestHeader) -> Vec<u8> {
        let fd = header.fd as u64;
        
        let file_size = match self.open_files.get(&fd) {
            Some(f) => {
                match self.get_inode_size(f.mount_id, f.inode) {
                    Ok(s) => s,
                    Err(e) => return self.error_response(header.request_id, e),
                }
            }
            None => return self.error_response(header.request_id, VfsError::InvalidFd),
        };
        
        let seek_from = VfsSeekFrom::from(header.flags as u8);
        let offset = header.offset as i64;
        
        let new_pos = match seek_from {
            VfsSeekFrom::Start => offset as u64,
            VfsSeekFrom::Current => {
                let current = self.open_files.get(&fd).map(|f| f.position).unwrap_or(0);
                (current as i64 + offset) as u64
            }
            VfsSeekFrom::End => (file_size as i64 + offset) as u64,
        };
        
        if let Some(f) = self.open_files.get_mut(&fd) {
            f.position = new_pos;
        }
        
        self.success_response(header.request_id, new_pos as i32)
    }

    /// Handle SYNC operation
    fn handle_sync(&mut self, header: &VfsRequestHeader) -> Vec<u8> {
        // Sync all dirty inodes
        // For RamFS, this is a no-op
        // For block-backed filesystems, flush to disk
        self.success_response(header.request_id, 0)
    }

    /// Handle RENAME operation  
    fn handle_rename(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        // Path contains both old and new paths separated by null
        let paths = match parse_path(msg, header) {
            Some(p) => p,
            None => return self.error_response(header.request_id, VfsError::InvalidPath),
        };
        
        let parts: Vec<&str> = paths.split('\0').collect();
        if parts.len() < 2 {
            return self.error_response(header.request_id, VfsError::InvalidArgument);
        }
        
        match self.rename_path(parts[0], parts[1]) {
            Ok(_) => self.success_response(header.request_id, 0),
            Err(e) => self.error_response(header.request_id, e),
        }
    }

    /// Handle TRUNCATE operation
    fn handle_truncate(&mut self, header: &VfsRequestHeader) -> Vec<u8> {
        let fd = header.fd as u64;
        
        let (mount_id, inode) = match self.open_files.get(&fd) {
            Some(f) => (f.mount_id, f.inode),
            None => return self.error_response(header.request_id, VfsError::InvalidFd),
        };
        
        match self.truncate_file(mount_id, inode, header.offset) {
            Ok(_) => self.success_response(header.request_id, 0),
            Err(e) => self.error_response(header.request_id, e),
        }
    }

    /// Handle CHMOD operation
    fn handle_chmod(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        let path = match parse_path(msg, header) {
            Some(p) => p,
            None => return self.error_response(header.request_id, VfsError::InvalidPath),
        };
        
        let mode = header.flags as u16;
        
        match self.change_mode(&path, mode, header.uid) {
            Ok(_) => self.success_response(header.request_id, 0),
            Err(e) => self.error_response(header.request_id, e),
        }
    }

    /// Handle CHOWN operation
    fn handle_chown(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        let path = match parse_path(msg, header) {
            Some(p) => p,
            None => return self.error_response(header.request_id, VfsError::InvalidPath),
        };
        
        // flags contains uid, length contains gid
        let new_uid = header.flags;
        let new_gid = header.length;
        
        match self.change_owner(&path, new_uid, new_gid, header.uid) {
            Ok(_) => self.success_response(header.request_id, 0),
            Err(e) => self.error_response(header.request_id, e),
        }
    }

    /// Handle TOUCH operation
    fn handle_touch(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        let path = match parse_path(msg, header) {
            Some(p) => p,
            None => return self.error_response(header.request_id, VfsError::InvalidPath),
        };
        
        // Create file if it doesn't exist
        match self.resolve_path(&path) {
            Ok((mount_id, inode)) => {
                // File exists, update timestamps
                match self.update_timestamps(mount_id, inode) {
                    Ok(_) => self.success_response(header.request_id, 0),
                    Err(e) => self.error_response(header.request_id, e),
                }
            }
            Err(VfsError::NotFound) => {
                // Create new file
                match self.create_file(&path, header.uid, header.gid, 0o644) {
                    Ok(_) => self.success_response(header.request_id, 0),
                    Err(e) => self.error_response(header.request_id, e),
                }
            }
            Err(e) => self.error_response(header.request_id, e),
        }
    }

    /// Handle CHDIR operation
    fn handle_chdir(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        let path = match parse_path(msg, header) {
            Some(p) => p,
            None => return self.error_response(header.request_id, VfsError::InvalidPath),
        };
        
        // Verify path exists and is a directory
        let (mount_id, inode) = match self.resolve_path(&path) {
            Ok(r) => r,
            Err(e) => return self.error_response(header.request_id, e),
        };
        
        // Check if it's a directory
        match self.get_file_type(mount_id, inode) {
            Ok(VfsFileType::Directory) => {}
            Ok(_) => return self.error_response(header.request_id, VfsError::NotADirectory),
            Err(e) => return self.error_response(header.request_id, e),
        }
        
        // Update process CWD
        let pid = header.uid; // Using uid as pid placeholder
        self.process_cwd.insert(pid, path);
        
        self.success_response(header.request_id, 0)
    }

    /// Handle GETCWD operation
    fn handle_getcwd(&mut self, header: &VfsRequestHeader) -> Vec<u8> {
        let pid = header.uid; // Using uid as pid placeholder
        
        let cwd = self.process_cwd.get(&pid)
            .cloned()
            .unwrap_or_else(|| "/".to_string());
        
        self.data_response(header.request_id, cwd.len() as i32, cwd.as_bytes())
    }

    /// Handle MOUNT operation
    fn handle_mount(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        // Only root can mount
        if header.uid != 0 {
            return self.error_response(header.request_id, VfsError::PermissionDenied);
        }
        
        let path = match parse_path(msg, header) {
            Some(p) => p,
            None => return self.error_response(header.request_id, VfsError::InvalidPath),
        };
        
        // TODO: Implement actual mount logic
        self.error_response(header.request_id, VfsError::NotSupported)
    }

    /// Handle UNMOUNT operation
    fn handle_unmount(&mut self, header: &VfsRequestHeader, msg: &[u8]) -> Vec<u8> {
        // Only root can unmount
        if header.uid != 0 {
            return self.error_response(header.request_id, VfsError::PermissionDenied);
        }
        
        let path = match parse_path(msg, header) {
            Some(p) => p,
            None => return self.error_response(header.request_id, VfsError::InvalidPath),
        };
        
        // Cannot unmount root
        if path == "/" {
            return self.error_response(header.request_id, VfsError::PermissionDenied);
        }
        
        match self.mounts.remove(&path) {
            Some(_) => self.success_response(header.request_id, 0),
            None => self.error_response(header.request_id, VfsError::NotFound),
        }
    }

    // ========== Helper methods ==========

    /// Resolve a path to (mount_id, inode)
    fn resolve_path(&self, path: &str) -> Result<(u64, u64), VfsError> {
        // Check dentry cache first
        if let Some(entry) = self.dentry_cache.get(path) {
            if entry.valid {
                return Ok((entry.mount_id, entry.inode));
            }
        }
        
        // Find mount point
        let (mount_path, mount) = self.find_mount(path)?;
        
        // Get relative path within mount
        let rel_path = if path.len() > mount_path.len() {
            &path[mount_path.len()..]
        } else {
            "/"
        };
        
        // Resolve within filesystem
        match &mount.fs_data {
            FilesystemData::RamFS(data) => self.resolve_ramfs_path(mount.id, data, rel_path),
            FilesystemData::FAT32(_) => Err(VfsError::NotSupported),
            FilesystemData::Ext4(_) => Err(VfsError::NotSupported),
            FilesystemData::DebFS(_) => Err(VfsError::NotSupported),
        }
    }

    /// Find the mount point for a path
    fn find_mount(&self, path: &str) -> Result<(&str, &MountPoint), VfsError> {
        // Find longest matching mount path
        let mut best_match: Option<(&str, &MountPoint)> = None;
        let mut best_len = 0;
        
        for (mount_path, mount) in &self.mounts {
            if path.starts_with(mount_path.as_str()) && mount_path.len() > best_len {
                best_match = Some((mount_path.as_str(), mount));
                best_len = mount_path.len();
            }
        }
        
        best_match.ok_or(VfsError::NoFilesystem)
    }

    /// Resolve path within RamFS
    fn resolve_ramfs_path(&self, mount_id: u64, data: &RamFsData, path: &str) -> Result<(u64, u64), VfsError> {
        let mut current_inode = 1u64; // Root inode
        
        if path == "/" || path.is_empty() {
            return Ok((mount_id, current_inode));
        }
        
        for component in path.split('/').filter(|s| !s.is_empty()) {
            let inode = data.inodes.get(&current_inode).ok_or(VfsError::NotFound)?;
            
            if inode.file_type != VfsFileType::Directory {
                return Err(VfsError::NotADirectory);
            }
            
            current_inode = *inode.children.get(component).ok_or(VfsError::NotFound)?;
        }
        
        Ok((mount_id, current_inode))
    }

    /// Check access permissions
    fn check_access(&self, mount_id: u64, inode: u64, uid: u32, gid: u32, flags: u32) -> Result<(), VfsError> {
        // Root can access anything
        if uid == 0 {
            return Ok(());
        }
        
        // Get inode info
        let stat = self.get_stat(mount_id, inode)?;
        
        // Check based on flags
        let need_read = flags & VfsOpenFlags::Read as u32 != 0;
        let need_write = flags & VfsOpenFlags::Write as u32 != 0;
        
        // Owner check
        if stat.uid == uid {
            let owner_perms = (stat.permissions >> 6) & 0o7;
            if need_read && (owner_perms & 0o4 == 0) {
                return Err(VfsError::PermissionDenied);
            }
            if need_write && (owner_perms & 0o2 == 0) {
                return Err(VfsError::PermissionDenied);
            }
            return Ok(());
        }
        
        // Group check
        if stat.gid == gid {
            let group_perms = (stat.permissions >> 3) & 0o7;
            if need_read && (group_perms & 0o4 == 0) {
                return Err(VfsError::PermissionDenied);
            }
            if need_write && (group_perms & 0o2 == 0) {
                return Err(VfsError::PermissionDenied);
            }
            return Ok(());
        }
        
        // Other check
        let other_perms = stat.permissions & 0o7;
        if need_read && (other_perms & 0o4 == 0) {
            return Err(VfsError::PermissionDenied);
        }
        if need_write && (other_perms & 0o2 == 0) {
            return Err(VfsError::PermissionDenied);
        }
        
        Ok(())
    }

    /// Get stat info for an inode
    fn get_stat(&self, mount_id: u64, inode: u64) -> Result<VfsStat, VfsError> {
        let mount = self.mounts.values().find(|m| m.id == mount_id).ok_or(VfsError::NoFilesystem)?;
        
        match &mount.fs_data {
            FilesystemData::RamFS(data) => {
                let node = data.inodes.get(&inode).ok_or(VfsError::NotFound)?;
                Ok(VfsStat {
                    inode: node.inode,
                    size: node.size,
                    file_type: node.file_type as u8,
                    permissions: node.permissions,
                    uid: node.uid,
                    gid: node.gid,
                    atime: node.atime,
                    mtime: node.mtime,
                    ctime: node.ctime,
                })
            }
            _ => Err(VfsError::NotSupported),
        }
    }

    /// Get file type for an inode
    fn get_file_type(&self, mount_id: u64, inode: u64) -> Result<VfsFileType, VfsError> {
        let stat = self.get_stat(mount_id, inode)?;
        Ok(VfsFileType::from(stat.file_type))
    }

    /// Get inode size
    fn get_inode_size(&self, mount_id: u64, inode: u64) -> Result<u64, VfsError> {
        let stat = self.get_stat(mount_id, inode)?;
        Ok(stat.size)
    }

    /// Read data from inode
    fn read_inode_data(&self, mount_id: u64, inode: u64, offset: u64, len: usize) -> Result<Vec<u8>, VfsError> {
        let mount = self.mounts.values().find(|m| m.id == mount_id).ok_or(VfsError::NoFilesystem)?;
        
        match &mount.fs_data {
            FilesystemData::RamFS(data) => {
                let node = data.inodes.get(&inode).ok_or(VfsError::NotFound)?;
                
                if node.file_type == VfsFileType::Directory {
                    return Err(VfsError::IsADirectory);
                }
                
                let start = offset as usize;
                if start >= node.data.len() {
                    return Ok(Vec::new());
                }
                
                let end = core::cmp::min(start + len, node.data.len());
                Ok(node.data[start..end].to_vec())
            }
            _ => Err(VfsError::NotSupported),
        }
    }

    /// Write data to inode
    fn write_inode_data(&mut self, mount_id: u64, inode: u64, offset: u64, data: &[u8]) -> Result<usize, VfsError> {
        let mount = self.mounts.values_mut().find(|m| m.id == mount_id).ok_or(VfsError::NoFilesystem)?;
        
        if mount.read_only {
            return Err(VfsError::ReadOnly);
        }
        
        match &mut mount.fs_data {
            FilesystemData::RamFS(fs_data) => {
                let node = fs_data.inodes.get_mut(&inode).ok_or(VfsError::NotFound)?;
                
                if node.file_type == VfsFileType::Directory {
                    return Err(VfsError::IsADirectory);
                }
                
                let start = offset as usize;
                let end = start + data.len();
                
                // Extend if necessary
                if end > node.data.len() {
                    node.data.resize(end, 0);
                }
                
                node.data[start..end].copy_from_slice(data);
                node.size = node.data.len() as u64;
                
                Ok(data.len())
            }
            _ => Err(VfsError::NotSupported),
        }
    }

    /// Create a new file
    fn create_file(&mut self, path: &str, uid: u32, gid: u32, mode: u16) -> Result<(u64, u64), VfsError> {
        // Get parent directory
        let (parent_path, name) = self.split_path(path)?;
        let (mount_id, parent_inode) = self.resolve_path(&parent_path)?;
        
        let mount = self.mounts.values_mut().find(|m| m.id == mount_id).ok_or(VfsError::NoFilesystem)?;
        
        if mount.read_only {
            return Err(VfsError::ReadOnly);
        }
        
        match &mut mount.fs_data {
            FilesystemData::RamFS(data) => {
                // Check parent is directory
                let parent = data.inodes.get(&parent_inode).ok_or(VfsError::NotFound)?;
                if parent.file_type != VfsFileType::Directory {
                    return Err(VfsError::NotADirectory);
                }
                
                // Check file doesn't already exist
                if parent.children.contains_key(&name) {
                    return Err(VfsError::AlreadyExists);
                }
                
                // Create new inode
                let new_inode = data.next_inode;
                data.next_inode += 1;
                
                let new_node = RamFsInode {
                    inode: new_inode,
                    file_type: VfsFileType::Regular,
                    permissions: mode,
                    uid,
                    gid,
                    size: 0,
                    data: Vec::new(),
                    children: BTreeMap::new(),
                    atime: 0,
                    mtime: 0,
                    ctime: 0,
                };
                
                data.inodes.insert(new_inode, new_node);
                
                // Add to parent
                if let Some(parent) = data.inodes.get_mut(&parent_inode) {
                    parent.children.insert(name, new_inode);
                }
                
                Ok((mount_id, new_inode))
            }
            _ => Err(VfsError::NotSupported),
        }
    }

    /// Create a directory
    fn create_directory(&mut self, path: &str, uid: u32, gid: u32, mode: u16) -> Result<(u64, u64), VfsError> {
        let (parent_path, name) = self.split_path(path)?;
        let (mount_id, parent_inode) = self.resolve_path(&parent_path)?;
        
        let mount = self.mounts.values_mut().find(|m| m.id == mount_id).ok_or(VfsError::NoFilesystem)?;
        
        if mount.read_only {
            return Err(VfsError::ReadOnly);
        }
        
        match &mut mount.fs_data {
            FilesystemData::RamFS(data) => {
                // Create new inode
                let new_inode = data.next_inode;
                data.next_inode += 1;
                
                let mut new_node = RamFsInode {
                    inode: new_inode,
                    file_type: VfsFileType::Directory,
                    permissions: if mode == 0 { 0o755 } else { mode },
                    uid,
                    gid,
                    size: 0,
                    data: Vec::new(),
                    children: BTreeMap::new(),
                    atime: 0,
                    mtime: 0,
                    ctime: 0,
                };
                
                // Add . and .. entries
                new_node.children.insert(".".to_string(), new_inode);
                new_node.children.insert("..".to_string(), parent_inode);
                
                data.inodes.insert(new_inode, new_node);
                
                // Add to parent
                if let Some(parent) = data.inodes.get_mut(&parent_inode) {
                    parent.children.insert(name, new_inode);
                }
                
                Ok((mount_id, new_inode))
            }
            _ => Err(VfsError::NotSupported),
        }
    }

    /// Remove a file
    fn remove_file(&mut self, path: &str) -> Result<(), VfsError> {
        let (parent_path, name) = self.split_path(path)?;
        let (mount_id, parent_inode) = self.resolve_path(&parent_path)?;
        let (_, file_inode) = self.resolve_path(path)?;
        
        let mount = self.mounts.values_mut().find(|m| m.id == mount_id).ok_or(VfsError::NoFilesystem)?;
        
        if mount.read_only {
            return Err(VfsError::ReadOnly);
        }
        
        match &mut mount.fs_data {
            FilesystemData::RamFS(data) => {
                // Check it's a file
                let node = data.inodes.get(&file_inode).ok_or(VfsError::NotFound)?;
                if node.file_type == VfsFileType::Directory {
                    return Err(VfsError::IsADirectory);
                }
                
                // Remove from parent
                if let Some(parent) = data.inodes.get_mut(&parent_inode) {
                    parent.children.remove(&name);
                }
                
                // Remove inode
                data.inodes.remove(&file_inode);
                
                Ok(())
            }
            _ => Err(VfsError::NotSupported),
        }
    }

    /// Remove a directory
    fn remove_directory(&mut self, path: &str) -> Result<(), VfsError> {
        let (parent_path, name) = self.split_path(path)?;
        let (mount_id, parent_inode) = self.resolve_path(&parent_path)?;
        let (_, dir_inode) = self.resolve_path(path)?;
        
        let mount = self.mounts.values_mut().find(|m| m.id == mount_id).ok_or(VfsError::NoFilesystem)?;
        
        if mount.read_only {
            return Err(VfsError::ReadOnly);
        }
        
        match &mut mount.fs_data {
            FilesystemData::RamFS(data) => {
                // Check it's a directory
                let node = data.inodes.get(&dir_inode).ok_or(VfsError::NotFound)?;
                if node.file_type != VfsFileType::Directory {
                    return Err(VfsError::NotADirectory);
                }
                
                // Check it's empty (only . and ..)
                if node.children.len() > 2 {
                    return Err(VfsError::NotEmpty);
                }
                
                // Remove from parent
                if let Some(parent) = data.inodes.get_mut(&parent_inode) {
                    parent.children.remove(&name);
                }
                
                // Remove inode
                data.inodes.remove(&dir_inode);
                
                Ok(())
            }
            _ => Err(VfsError::NotSupported),
        }
    }

    /// List directory contents
    fn list_directory(&self, mount_id: u64, inode: u64) -> Result<Vec<(String, u64, VfsFileType)>, VfsError> {
        let mount = self.mounts.values().find(|m| m.id == mount_id).ok_or(VfsError::NoFilesystem)?;
        
        match &mount.fs_data {
            FilesystemData::RamFS(data) => {
                let node = data.inodes.get(&inode).ok_or(VfsError::NotFound)?;
                
                if node.file_type != VfsFileType::Directory {
                    return Err(VfsError::NotADirectory);
                }
                
                let mut entries = Vec::new();
                for (name, child_inode) in &node.children {
                    if let Some(child) = data.inodes.get(child_inode) {
                        entries.push((name.clone(), *child_inode, child.file_type));
                    }
                }
                
                Ok(entries)
            }
            _ => Err(VfsError::NotSupported),
        }
    }

    /// Rename a path
    fn rename_path(&mut self, _old_path: &str, _new_path: &str) -> Result<(), VfsError> {
        // TODO: Implement rename
        Err(VfsError::NotSupported)
    }

    /// Truncate a file
    fn truncate_file(&mut self, mount_id: u64, inode: u64, size: u64) -> Result<(), VfsError> {
        let mount = self.mounts.values_mut().find(|m| m.id == mount_id).ok_or(VfsError::NoFilesystem)?;
        
        if mount.read_only {
            return Err(VfsError::ReadOnly);
        }
        
        match &mut mount.fs_data {
            FilesystemData::RamFS(data) => {
                let node = data.inodes.get_mut(&inode).ok_or(VfsError::NotFound)?;
                node.data.truncate(size as usize);
                node.size = size;
                Ok(())
            }
            _ => Err(VfsError::NotSupported),
        }
    }

    /// Change file mode
    fn change_mode(&mut self, path: &str, mode: u16, uid: u32) -> Result<(), VfsError> {
        let (mount_id, inode) = self.resolve_path(path)?;
        
        let mount = self.mounts.values_mut().find(|m| m.id == mount_id).ok_or(VfsError::NoFilesystem)?;
        
        match &mut mount.fs_data {
            FilesystemData::RamFS(data) => {
                let node = data.inodes.get_mut(&inode).ok_or(VfsError::NotFound)?;
                
                // Only owner or root can change mode
                if uid != 0 && node.uid != uid {
                    return Err(VfsError::PermissionDenied);
                }
                
                node.permissions = mode;
                Ok(())
            }
            _ => Err(VfsError::NotSupported),
        }
    }

    /// Change file owner
    fn change_owner(&mut self, path: &str, new_uid: u32, new_gid: u32, uid: u32) -> Result<(), VfsError> {
        // Only root can change owner
        if uid != 0 {
            return Err(VfsError::PermissionDenied);
        }
        
        let (mount_id, inode) = self.resolve_path(path)?;
        
        let mount = self.mounts.values_mut().find(|m| m.id == mount_id).ok_or(VfsError::NoFilesystem)?;
        
        match &mut mount.fs_data {
            FilesystemData::RamFS(data) => {
                let node = data.inodes.get_mut(&inode).ok_or(VfsError::NotFound)?;
                node.uid = new_uid;
                node.gid = new_gid;
                Ok(())
            }
            _ => Err(VfsError::NotSupported),
        }
    }

    /// Update timestamps
    fn update_timestamps(&mut self, mount_id: u64, inode: u64) -> Result<(), VfsError> {
        let mount = self.mounts.values_mut().find(|m| m.id == mount_id).ok_or(VfsError::NoFilesystem)?;
        
        match &mut mount.fs_data {
            FilesystemData::RamFS(data) => {
                let node = data.inodes.get_mut(&inode).ok_or(VfsError::NotFound)?;
                // In a real implementation, we'd get the current time
                node.atime = 0;
                node.mtime = 0;
                Ok(())
            }
            _ => Err(VfsError::NotSupported),
        }
    }

    /// Split path into parent and name
    fn split_path(&self, path: &str) -> Result<(String, String), VfsError> {
        let path = path.trim_end_matches('/');
        
        if let Some(idx) = path.rfind('/') {
            let parent = if idx == 0 { "/".to_string() } else { path[..idx].to_string() };
            let name = path[idx+1..].to_string();
            if name.is_empty() {
                return Err(VfsError::InvalidPath);
            }
            Ok((parent, name))
        } else {
            Err(VfsError::InvalidPath)
        }
    }

    // ========== Response builders ==========

    fn success_response(&self, request_id: u32, result: i32) -> Vec<u8> {
        let header = VfsResponseHeader::success(request_id, result);
        header.to_bytes().to_vec()
    }

    fn error_response(&self, request_id: u32, error: VfsError) -> Vec<u8> {
        let header = VfsResponseHeader::error(request_id, error);
        header.to_bytes().to_vec()
    }

    fn data_response(&self, request_id: u32, result: i32, data: &[u8]) -> Vec<u8> {
        let header = VfsResponseHeader::success_with_data(request_id, result, data.len() as u32);
        let mut response = header.to_bytes().to_vec();
        response.extend_from_slice(data);
        response
    }

    fn stat_response(&self, request_id: u32, stat: &VfsStat) -> Vec<u8> {
        let header = VfsResponseHeader::success_with_data(request_id, 0, VfsStat::SIZE as u32);
        let mut response = header.to_bytes().to_vec();
        response.extend_from_slice(&stat.to_bytes());
        response
    }

    fn dirlist_response(&self, request_id: u32, entries: &[(String, u64, VfsFileType)]) -> Vec<u8> {
        // Encode directory entries
        let mut data = Vec::new();
        
        for (name, inode, file_type) in entries {
            // Entry header
            let entry = VfsDirEntry {
                inode: *inode,
                file_type: *file_type as u8,
                name_len: name.len() as u16,
            };
            data.extend_from_slice(&entry.inode.to_le_bytes());
            data.push(entry.file_type);
            data.extend_from_slice(&entry.name_len.to_le_bytes());
            data.extend_from_slice(name.as_bytes());
        }
        
        let header = VfsResponseHeader::success_with_data(request_id, entries.len() as i32, data.len() as u32);
        let mut response = header.to_bytes().to_vec();
        response.extend_from_slice(&data);
        response
    }
}

impl From<u8> for VfsFileType {
    fn from(v: u8) -> Self {
        match v {
            1 => VfsFileType::Regular,
            2 => VfsFileType::Directory,
            3 => VfsFileType::Symlink,
            _ => VfsFileType::Regular,
        }
    }
}

impl From<u8> for VfsSeekFrom {
    fn from(v: u8) -> Self {
        match v {
            0 => VfsSeekFrom::Start,
            1 => VfsSeekFrom::Current,
            2 => VfsSeekFrom::End,
            _ => VfsSeekFrom::Start,
        }
    }
}

// ========== Entry Point ==========

/// VFS Server entry point
/// 
/// This is called by the kernel after the server is loaded into memory.
/// The server registers on the VFS endpoint and loops forever handling requests.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Initialize VFS server
    let mut server = VfsServer::new();
    server.init_root_ramfs();
    
    // Create standard directories
    let _ = server.create_directory("/bin", 0, 0, 0o755);
    let _ = server.create_directory("/etc", 0, 0, 0o755);
    let _ = server.create_directory("/home", 0, 0, 0o755);
    let _ = server.create_directory("/home/debos", 1000, 1000, 0o755);
    let _ = server.create_directory("/tmp", 0, 0, 0o1777);
    let _ = server.create_directory("/var", 0, 0, 0o755);
    let _ = server.create_directory("/var/log", 0, 0, 0o755);
    let _ = server.create_directory("/dev", 0, 0, 0o755);
    let _ = server.create_directory("/proc", 0, 0, 0o555);
    let _ = server.create_directory("/sys", 0, 0, 0o555);
    
    // TODO: Register on VFS_ENDPOINT_ID via IPC
    // TODO: Main loop - wait for IPC messages, handle them, reply
    
    // For now, just loop
    loop {
        // In real implementation:
        // 1. let (msg, sender) = ipc_wait(VFS_ENDPOINT_ID, &mut buffer);
        // 2. let response = server.handle_request(&msg);
        // 3. ipc_reply(VFS_ENDPOINT_ID, sender, &response);
        
        // Placeholder: spin
        core::hint::spin_loop();
    }
}

/// Panic handler for the VFS server
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

// Protocol module - reusing the kernel's VFS protocol
mod protocol {
    pub use super::*;
    
    // Re-export all protocol types
    pub const MAX_PATH_LEN: usize = 1024;
    pub const MAX_NAME_LEN: usize = 256;
    pub const MAX_DATA_LEN: usize = 3072;
    pub const VFS_ENDPOINT_ID: u64 = 1000;
    
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VfsOp {
        Open = 1,
        Close = 2,
        Read = 3,
        Write = 4,
        Stat = 5,
        Mkdir = 6,
        Rmdir = 7,
        Unlink = 8,
        Readdir = 9,
        Seek = 10,
        Sync = 11,
        Rename = 12,
        Symlink = 13,
        Readlink = 14,
        Truncate = 15,
        Chmod = 16,
        Chown = 17,
        Touch = 18,
        Chdir = 19,
        Getcwd = 20,
        Mount = 100,
        Unmount = 101,
        Invalid = 255,
    }
    
    impl From<u8> for VfsOp {
        fn from(v: u8) -> Self {
            match v {
                1 => VfsOp::Open, 2 => VfsOp::Close, 3 => VfsOp::Read, 4 => VfsOp::Write,
                5 => VfsOp::Stat, 6 => VfsOp::Mkdir, 7 => VfsOp::Rmdir, 8 => VfsOp::Unlink,
                9 => VfsOp::Readdir, 10 => VfsOp::Seek, 11 => VfsOp::Sync, 12 => VfsOp::Rename,
                13 => VfsOp::Symlink, 14 => VfsOp::Readlink, 15 => VfsOp::Truncate,
                16 => VfsOp::Chmod, 17 => VfsOp::Chown, 18 => VfsOp::Touch,
                19 => VfsOp::Chdir, 20 => VfsOp::Getcwd, 100 => VfsOp::Mount,
                101 => VfsOp::Unmount, _ => VfsOp::Invalid,
            }
        }
    }
    
    #[repr(u32)]
    #[derive(Debug, Clone, Copy)]
    pub enum VfsOpenFlags {
        Read = 0x01,
        Write = 0x02,
        Create = 0x04,
        Append = 0x08,
        Truncate = 0x10,
    }
    
    #[repr(u8)]
    #[derive(Debug, Clone, Copy)]
    pub enum VfsSeekFrom {
        Start = 0,
        Current = 1,
        End = 2,
    }
    
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VfsFileType {
        Regular = 1,
        Directory = 2,
        Symlink = 3,
    }
    
    #[repr(i32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VfsError {
        Success = 0,
        NotFound = -1,
        AlreadyExists = -2,
        NotADirectory = -3,
        IsADirectory = -4,
        NotEmpty = -5,
        PermissionDenied = -6,
        InvalidPath = -7,
        NoSpace = -8,
        ReadOnly = -9,
        IoError = -10,
        InvalidFd = -11,
        TooManyOpenFiles = -12,
        InvalidArgument = -13,
        InvalidFilesystem = -14,
        NoFilesystem = -15,
        NotSupported = -16,
        Unknown = -255,
    }
    
    #[repr(C, packed)]
    #[derive(Debug, Clone, Copy)]
    pub struct VfsRequestHeader {
        pub op: u8,
        pub request_id: u32,
        pub uid: u32,
        pub gid: u32,
        pub fd: i32,
        pub flags: u32,
        pub offset: u64,
        pub length: u32,
        pub path_len: u32,
    }
    
    impl VfsRequestHeader {
        pub const SIZE: usize = core::mem::size_of::<Self>();
        
        pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
            if bytes.len() < Self::SIZE { return None; }
            let mut header = [0u8; Self::SIZE];
            header.copy_from_slice(&bytes[..Self::SIZE]);
            Some(unsafe { core::mem::transmute(header) })
        }
    }
    
    #[repr(C, packed)]
    #[derive(Debug, Clone, Copy)]
    pub struct VfsResponseHeader {
        pub request_id: u32,
        pub error: i32,
        pub result: i32,
        pub data_len: u32,
    }
    
    impl VfsResponseHeader {
        pub const SIZE: usize = core::mem::size_of::<Self>();
        
        pub fn success(request_id: u32, result: i32) -> Self {
            Self { request_id, error: VfsError::Success as i32, result, data_len: 0 }
        }
        
        pub fn success_with_data(request_id: u32, result: i32, data_len: u32) -> Self {
            Self { request_id, error: VfsError::Success as i32, result, data_len }
        }
        
        pub fn error(request_id: u32, error: VfsError) -> Self {
            Self { request_id, error: error as i32, result: -1, data_len: 0 }
        }
        
        pub fn to_bytes(&self) -> [u8; Self::SIZE] {
            unsafe { core::mem::transmute_copy(self) }
        }
    }
    
    #[repr(C, packed)]
    #[derive(Debug, Clone, Copy)]
    pub struct VfsStat {
        pub inode: u64,
        pub size: u64,
        pub file_type: u8,
        pub permissions: u16,
        pub uid: u32,
        pub gid: u32,
        pub atime: u64,
        pub mtime: u64,
        pub ctime: u64,
    }
    
    impl VfsStat {
        pub const SIZE: usize = core::mem::size_of::<Self>();
        
        pub fn to_bytes(&self) -> [u8; Self::SIZE] {
            unsafe { core::mem::transmute_copy(self) }
        }
    }
    
    #[repr(C, packed)]
    #[derive(Debug, Clone, Copy)]
    pub struct VfsDirEntry {
        pub inode: u64,
        pub file_type: u8,
        pub name_len: u16,
    }
    
    impl VfsDirEntry {
        pub const SIZE: usize = core::mem::size_of::<Self>();
    }
    
    pub fn parse_path(msg: &[u8], header: &VfsRequestHeader) -> Option<String> {
        if header.path_len == 0 { return None; }
        let path_start = VfsRequestHeader::SIZE;
        let path_end = path_start + header.path_len as usize;
        if msg.len() < path_end { return None; }
        let path_bytes = &msg[path_start..path_end];
        let path_bytes = if path_bytes.last() == Some(&0) {
            &path_bytes[..path_bytes.len() - 1]
        } else {
            path_bytes
        };
        String::from_utf8(path_bytes.to_vec()).ok()
    }
    
    pub fn parse_data(msg: &[u8], header: &VfsRequestHeader) -> Option<&[u8]> {
        let data_start = VfsRequestHeader::SIZE + header.path_len as usize;
        let data_len = header.length as usize;
        if data_len == 0 || msg.len() < data_start + data_len { return None; }
        Some(&msg[data_start..data_start + data_len])
    }
}
