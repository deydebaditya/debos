//! Filesystem API
//!
//! This module provides the userspace filesystem API for DebOS applications.
//! All operations are routed via IPC to the VFS server.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use libdebos::fs::{open, read, write, close, OpenFlags};
//!
//! // Open a file for reading
//! let fd = open("/etc/passwd", OpenFlags::READ)?;
//!
//! // Read contents
//! let mut buf = [0u8; 1024];
//! let bytes = read(fd, &mut buf)?;
//!
//! // Close the file
//! close(fd)?;
//! ```

use core::sync::atomic::{AtomicU32, Ordering};

/// VFS Server endpoint ID
const VFS_ENDPOINT_ID: u64 = 1000;

/// Maximum path length
pub const MAX_PATH: usize = 1024;

/// File open flags
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum OpenFlags {
    /// Open for reading
    Read = 0x01,
    /// Open for writing
    Write = 0x02,
    /// Create file if it doesn't exist
    Create = 0x04,
    /// Append to file
    Append = 0x08,
    /// Truncate file on open
    Truncate = 0x10,
    /// Read and write
    ReadWrite = 0x03,
    /// Create and truncate
    CreateTrunc = 0x16,
}

/// Seek origin
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SeekFrom {
    /// From beginning of file
    Start = 0,
    /// From current position
    Current = 1,
    /// From end of file
    End = 2,
}

/// File type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FileType {
    /// Regular file
    Regular = 1,
    /// Directory
    Directory = 2,
    /// Symbolic link
    Symlink = 3,
}

/// File metadata
#[derive(Debug, Clone, Copy)]
pub struct Stat {
    /// Inode number
    pub inode: u64,
    /// File size in bytes
    pub size: u64,
    /// File type
    pub file_type: FileType,
    /// Permission bits
    pub permissions: u16,
    /// Owner user ID
    pub uid: u32,
    /// Owner group ID
    pub gid: u32,
    /// Last access time
    pub atime: u64,
    /// Last modification time
    pub mtime: u64,
    /// Creation time
    pub ctime: u64,
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Entry name
    pub name: [u8; 256],
    /// Name length
    pub name_len: usize,
    /// Inode number
    pub inode: u64,
    /// File type
    pub file_type: FileType,
}

impl DirEntry {
    /// Get name as str
    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }
}

/// Filesystem error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FsError {
    /// Success (not an error)
    Success = 0,
    /// File or directory not found
    NotFound = -1,
    /// File or directory already exists
    AlreadyExists = -2,
    /// Path is not a directory
    NotADirectory = -3,
    /// Path is a directory
    IsADirectory = -4,
    /// Directory not empty
    NotEmpty = -5,
    /// Permission denied
    PermissionDenied = -6,
    /// Invalid path
    InvalidPath = -7,
    /// No space left on device
    NoSpace = -8,
    /// Read-only filesystem
    ReadOnly = -9,
    /// I/O error
    IoError = -10,
    /// Invalid file descriptor
    InvalidFd = -11,
    /// Too many open files
    TooManyOpenFiles = -12,
    /// Invalid argument
    InvalidArgument = -13,
}

impl From<i32> for FsError {
    fn from(v: i32) -> Self {
        match v {
            0 => FsError::Success,
            -1 => FsError::NotFound,
            -2 => FsError::AlreadyExists,
            -3 => FsError::NotADirectory,
            -4 => FsError::IsADirectory,
            -5 => FsError::NotEmpty,
            -6 => FsError::PermissionDenied,
            -7 => FsError::InvalidPath,
            -8 => FsError::NoSpace,
            -9 => FsError::ReadOnly,
            -10 => FsError::IoError,
            -11 => FsError::InvalidFd,
            -12 => FsError::TooManyOpenFiles,
            -13 => FsError::InvalidArgument,
            _ => FsError::IoError,
        }
    }
}

/// Filesystem result type
pub type FsResult<T> = Result<T, FsError>;

// ========== VFS Protocol Structures ==========

/// VFS operation codes
#[repr(u8)]
#[derive(Clone, Copy)]
enum VfsOp {
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
    Touch = 18,
    Chdir = 19,
    Getcwd = 20,
}

/// Request ID counter
static REQUEST_ID: AtomicU32 = AtomicU32::new(1);

/// VFS request header (37 bytes)
#[repr(C, packed)]
struct VfsRequestHeader {
    op: u8,
    request_id: u32,
    uid: u32,
    gid: u32,
    fd: i32,
    flags: u32,
    offset: u64,
    length: u32,
    path_len: u32,
}

impl VfsRequestHeader {
    const SIZE: usize = 37;
    
    fn new(op: VfsOp) -> Self {
        Self {
            op: op as u8,
            request_id: REQUEST_ID.fetch_add(1, Ordering::Relaxed),
            uid: 1000, // TODO: Get from process credentials
            gid: 1000,
            fd: -1,
            flags: 0,
            offset: 0,
            length: 0,
            path_len: 0,
        }
    }
    
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        unsafe { core::mem::transmute_copy(self) }
    }
}

/// VFS response header (16 bytes)
#[repr(C, packed)]
struct VfsResponseHeader {
    request_id: u32,
    error: i32,
    result: i32,
    data_len: u32,
}

impl VfsResponseHeader {
    const SIZE: usize = 16;
    
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let mut header = [0u8; Self::SIZE];
        header.copy_from_slice(&bytes[..Self::SIZE]);
        Some(unsafe { core::mem::transmute(header) })
    }
}

// ========== IPC Helper ==========

fn vfs_call(request: &[u8], reply: &mut [u8]) -> FsResult<usize> {
    // Use IPC syscall to communicate with VFS server
    let result = unsafe {
        crate::syscall::syscall(
            crate::syscall::SYS_IPC_CALL,
            VFS_ENDPOINT_ID,
            request.as_ptr() as u64,
            request.len() as u64,
            reply.as_mut_ptr() as u64,
            reply.len() as u64,
            0,
        )
    };
    
    if result < 0 {
        return Err(FsError::IoError);
    }
    
    // Check response header
    if let Some(header) = VfsResponseHeader::from_bytes(reply) {
        if header.error != 0 {
            return Err(FsError::from(header.error));
        }
        Ok(result as usize)
    } else {
        Err(FsError::IoError)
    }
}

fn build_request(header: &VfsRequestHeader, path: Option<&str>, data: Option<&[u8]>) -> ([u8; 4096], usize) {
    let mut buf = [0u8; 4096];
    let mut offset = 0;
    
    // Write header
    let header_bytes = header.to_bytes();
    buf[..VfsRequestHeader::SIZE].copy_from_slice(&header_bytes);
    offset += VfsRequestHeader::SIZE;
    
    // Write path if present
    if let Some(p) = path {
        let bytes = p.as_bytes();
        buf[offset..offset + bytes.len()].copy_from_slice(bytes);
        offset += bytes.len();
        buf[offset] = 0; // null terminator
        offset += 1;
    }
    
    // Write data if present
    if let Some(d) = data {
        buf[offset..offset + d.len()].copy_from_slice(d);
        offset += d.len();
    }
    
    (buf, offset)
}

// ========== Public API ==========

/// Open a file
///
/// # Arguments
/// * `path` - Path to the file
/// * `flags` - Open flags
///
/// # Returns
/// File descriptor on success, error on failure
pub fn open(path: &str, flags: OpenFlags) -> FsResult<i32> {
    let mut header = VfsRequestHeader::new(VfsOp::Open);
    header.flags = flags as u32;
    header.path_len = path.len() as u32 + 1;
    
    let (request, len) = build_request(&header, Some(path), None);
    let mut reply = [0u8; 256];
    
    vfs_call(&request[..len], &mut reply)?;
    
    if let Some(resp) = VfsResponseHeader::from_bytes(&reply) {
        Ok(resp.result)
    } else {
        Err(FsError::IoError)
    }
}

/// Close a file
///
/// # Arguments
/// * `fd` - File descriptor
pub fn close(fd: i32) -> FsResult<()> {
    let mut header = VfsRequestHeader::new(VfsOp::Close);
    header.fd = fd;
    
    let (request, len) = build_request(&header, None, None);
    let mut reply = [0u8; 256];
    
    vfs_call(&request[..len], &mut reply)?;
    Ok(())
}

/// Read from a file
///
/// # Arguments
/// * `fd` - File descriptor
/// * `buf` - Buffer to read into
///
/// # Returns
/// Number of bytes read
pub fn read(fd: i32, buf: &mut [u8]) -> FsResult<usize> {
    let mut header = VfsRequestHeader::new(VfsOp::Read);
    header.fd = fd;
    header.length = buf.len() as u32;
    
    let (request, len) = build_request(&header, None, None);
    let mut reply = [0u8; 4096];
    
    let reply_len = vfs_call(&request[..len], &mut reply)?;
    
    if let Some(resp) = VfsResponseHeader::from_bytes(&reply) {
        let data_start = VfsResponseHeader::SIZE;
        let data_len = resp.data_len as usize;
        
        if reply_len >= data_start + data_len {
            let copy_len = core::cmp::min(data_len, buf.len());
            buf[..copy_len].copy_from_slice(&reply[data_start..data_start + copy_len]);
            Ok(copy_len)
        } else {
            Err(FsError::IoError)
        }
    } else {
        Err(FsError::IoError)
    }
}

/// Write to a file
///
/// # Arguments
/// * `fd` - File descriptor
/// * `data` - Data to write
///
/// # Returns
/// Number of bytes written
pub fn write(fd: i32, data: &[u8]) -> FsResult<usize> {
    let mut header = VfsRequestHeader::new(VfsOp::Write);
    header.fd = fd;
    header.length = data.len() as u32;
    
    let (request, len) = build_request(&header, None, Some(data));
    let mut reply = [0u8; 256];
    
    vfs_call(&request[..len], &mut reply)?;
    
    if let Some(resp) = VfsResponseHeader::from_bytes(&reply) {
        Ok(resp.result as usize)
    } else {
        Err(FsError::IoError)
    }
}

/// Get file status
///
/// # Arguments
/// * `path` - Path to the file
pub fn stat(path: &str) -> FsResult<Stat> {
    let mut header = VfsRequestHeader::new(VfsOp::Stat);
    header.path_len = path.len() as u32 + 1;
    
    let (request, len) = build_request(&header, Some(path), None);
    let mut reply = [0u8; 256];
    
    vfs_call(&request[..len], &mut reply)?;
    
    // Parse VfsStat from response
    // VfsStat is at offset VfsResponseHeader::SIZE
    let stat_start = VfsResponseHeader::SIZE;
    
    if reply.len() < stat_start + 49 {
        return Err(FsError::IoError);
    }
    
    // Parse fields
    let inode = u64::from_le_bytes(reply[stat_start..stat_start+8].try_into().unwrap());
    let size = u64::from_le_bytes(reply[stat_start+8..stat_start+16].try_into().unwrap());
    let file_type = match reply[stat_start + 16] {
        1 => FileType::Regular,
        2 => FileType::Directory,
        3 => FileType::Symlink,
        _ => FileType::Regular,
    };
    let permissions = u16::from_le_bytes(reply[stat_start+17..stat_start+19].try_into().unwrap());
    let uid = u32::from_le_bytes(reply[stat_start+19..stat_start+23].try_into().unwrap());
    let gid = u32::from_le_bytes(reply[stat_start+23..stat_start+27].try_into().unwrap());
    let atime = u64::from_le_bytes(reply[stat_start+27..stat_start+35].try_into().unwrap());
    let mtime = u64::from_le_bytes(reply[stat_start+35..stat_start+43].try_into().unwrap());
    let ctime = u64::from_le_bytes(reply[stat_start+43..stat_start+51].try_into().unwrap());
    
    Ok(Stat {
        inode,
        size,
        file_type,
        permissions,
        uid,
        gid,
        atime,
        mtime,
        ctime,
    })
}

/// Create a directory
///
/// # Arguments
/// * `path` - Path to the directory
pub fn mkdir(path: &str) -> FsResult<()> {
    let mut header = VfsRequestHeader::new(VfsOp::Mkdir);
    header.path_len = path.len() as u32 + 1;
    header.flags = 0o755;
    
    let (request, len) = build_request(&header, Some(path), None);
    let mut reply = [0u8; 256];
    
    vfs_call(&request[..len], &mut reply)?;
    Ok(())
}

/// Remove a directory
///
/// # Arguments
/// * `path` - Path to the directory
pub fn rmdir(path: &str) -> FsResult<()> {
    let mut header = VfsRequestHeader::new(VfsOp::Rmdir);
    header.path_len = path.len() as u32 + 1;
    
    let (request, len) = build_request(&header, Some(path), None);
    let mut reply = [0u8; 256];
    
    vfs_call(&request[..len], &mut reply)?;
    Ok(())
}

/// Remove a file
///
/// # Arguments
/// * `path` - Path to the file
pub fn unlink(path: &str) -> FsResult<()> {
    let mut header = VfsRequestHeader::new(VfsOp::Unlink);
    header.path_len = path.len() as u32 + 1;
    
    let (request, len) = build_request(&header, Some(path), None);
    let mut reply = [0u8; 256];
    
    vfs_call(&request[..len], &mut reply)?;
    Ok(())
}

/// Seek in a file
///
/// # Arguments
/// * `fd` - File descriptor
/// * `offset` - Offset to seek to
/// * `from` - Seek origin
///
/// # Returns
/// New file position
pub fn seek(fd: i32, offset: i64, from: SeekFrom) -> FsResult<u64> {
    let mut header = VfsRequestHeader::new(VfsOp::Seek);
    header.fd = fd;
    header.flags = from as u32;
    header.offset = offset as u64;
    
    let (request, len) = build_request(&header, None, None);
    let mut reply = [0u8; 256];
    
    vfs_call(&request[..len], &mut reply)?;
    
    if let Some(resp) = VfsResponseHeader::from_bytes(&reply) {
        Ok(resp.result as u64)
    } else {
        Err(FsError::IoError)
    }
}

/// Sync file to disk
///
/// # Arguments
/// * `fd` - File descriptor
pub fn sync(fd: i32) -> FsResult<()> {
    let mut header = VfsRequestHeader::new(VfsOp::Sync);
    header.fd = fd;
    
    let (request, len) = build_request(&header, None, None);
    let mut reply = [0u8; 256];
    
    vfs_call(&request[..len], &mut reply)?;
    Ok(())
}

/// Rename a file or directory
///
/// # Arguments
/// * `old_path` - Current path
/// * `new_path` - New path
pub fn rename(old_path: &str, new_path: &str) -> FsResult<()> {
    let mut header = VfsRequestHeader::new(VfsOp::Rename);
    
    // Combine paths with null separator
    let combined = {
        let mut s = [0u8; 2048];
        let old_bytes = old_path.as_bytes();
        let new_bytes = new_path.as_bytes();
        s[..old_bytes.len()].copy_from_slice(old_bytes);
        s[old_bytes.len()] = 0;
        s[old_bytes.len() + 1..old_bytes.len() + 1 + new_bytes.len()].copy_from_slice(new_bytes);
        (s, old_bytes.len() + 1 + new_bytes.len() + 1)
    };
    
    header.path_len = combined.1 as u32;
    
    let (request, len) = build_request(&header, None, Some(&combined.0[..combined.1]));
    let mut reply = [0u8; 256];
    
    vfs_call(&request[..len], &mut reply)?;
    Ok(())
}

/// Create/touch a file
///
/// # Arguments
/// * `path` - Path to the file
pub fn touch(path: &str) -> FsResult<()> {
    let mut header = VfsRequestHeader::new(VfsOp::Touch);
    header.path_len = path.len() as u32 + 1;
    
    let (request, len) = build_request(&header, Some(path), None);
    let mut reply = [0u8; 256];
    
    vfs_call(&request[..len], &mut reply)?;
    Ok(())
}

/// Change current directory
///
/// # Arguments
/// * `path` - Path to the directory
pub fn chdir(path: &str) -> FsResult<()> {
    let mut header = VfsRequestHeader::new(VfsOp::Chdir);
    header.path_len = path.len() as u32 + 1;
    
    let (request, len) = build_request(&header, Some(path), None);
    let mut reply = [0u8; 256];
    
    vfs_call(&request[..len], &mut reply)?;
    Ok(())
}

/// Get current directory
///
/// # Returns
/// Current directory path in the provided buffer
pub fn getcwd(buf: &mut [u8]) -> FsResult<usize> {
    let header = VfsRequestHeader::new(VfsOp::Getcwd);
    
    let (request, len) = build_request(&header, None, None);
    let mut reply = [0u8; 1024];
    
    vfs_call(&request[..len], &mut reply)?;
    
    if let Some(resp) = VfsResponseHeader::from_bytes(&reply) {
        let data_start = VfsResponseHeader::SIZE;
        let data_len = resp.data_len as usize;
        
        let copy_len = core::cmp::min(data_len, buf.len());
        buf[..copy_len].copy_from_slice(&reply[data_start..data_start + copy_len]);
        Ok(copy_len)
    } else {
        Err(FsError::IoError)
    }
}

/// Read directory entries
///
/// # Arguments
/// * `fd` - Directory file descriptor
/// * `entries` - Buffer to store entries
///
/// # Returns
/// Number of entries read
pub fn readdir(fd: i32, entries: &mut [DirEntry]) -> FsResult<usize> {
    let mut header = VfsRequestHeader::new(VfsOp::Readdir);
    header.fd = fd;
    
    let (request, len) = build_request(&header, None, None);
    let mut reply = [0u8; 4096];
    
    vfs_call(&request[..len], &mut reply)?;
    
    if let Some(resp) = VfsResponseHeader::from_bytes(&reply) {
        let num_entries = resp.result as usize;
        let mut offset = VfsResponseHeader::SIZE;
        let mut count = 0;
        
        for entry in entries.iter_mut().take(num_entries) {
            if offset + 11 > reply.len() {
                break;
            }
            
            entry.inode = u64::from_le_bytes(reply[offset..offset+8].try_into().unwrap_or([0;8]));
            entry.file_type = match reply[offset + 8] {
                1 => FileType::Regular,
                2 => FileType::Directory,
                3 => FileType::Symlink,
                _ => FileType::Regular,
            };
            entry.name_len = u16::from_le_bytes(reply[offset+9..offset+11].try_into().unwrap_or([0;2])) as usize;
            
            offset += 11;
            
            if offset + entry.name_len > reply.len() || entry.name_len > 256 {
                break;
            }
            
            entry.name = [0u8; 256];
            entry.name[..entry.name_len].copy_from_slice(&reply[offset..offset+entry.name_len]);
            offset += entry.name_len;
            
            count += 1;
        }
        
        Ok(count)
    } else {
        Err(FsError::IoError)
    }
}
