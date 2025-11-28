//! VFS Client - Kernel-side IPC bridge to VFS Server
//!
//! This module provides a bridge between in-kernel filesystem operations
//! and the userspace VFS server. During early boot (before the VFS server
//! is running), operations fall back to the in-kernel RamFS.
//!
//! ## Operation Modes
//!
//! 1. **Early Boot Mode**: Direct in-kernel RamFS (no IPC)
//! 2. **Normal Mode**: All operations go through IPC to VFS server
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Kernel Filesystem API                         │
//! │              (fs::open, fs::read, fs::write, etc.)              │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        VFS Client                                │
//! │  ┌─────────────────┐              ┌─────────────────────────┐   │
//! │  │  Early Boot?    │──Yes────────▶│   In-kernel RamFS       │   │
//! │  └────────┬────────┘              └─────────────────────────┘   │
//! │           │No                                                    │
//! │           ▼                                                      │
//! │  ┌─────────────────────────────────────────────────────────┐    │
//! │  │            IPC to VFS Server                             │    │
//! │  │  1. Build VfsRequestHeader                               │    │
//! │  │  2. ipc_call(VFS_ENDPOINT, request, reply)              │    │
//! │  │  3. Parse VfsResponseHeader                              │    │
//! │  └─────────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use super::vfs_protocol::*;
use super::{FsError, FsResult, OpenFlags, Stat, InodeType, DirEntry, SeekFrom};
use crate::ipc::{EndpointId, IpcError};

/// Whether the VFS server is running and accepting IPC
static VFS_SERVER_READY: AtomicBool = AtomicBool::new(false);

/// VFS server endpoint ID
const VFS_ENDPOINT: EndpointId = EndpointId(VFS_ENDPOINT_ID);

/// Mark the VFS server as ready to accept IPC
pub fn set_vfs_server_ready(ready: bool) {
    VFS_SERVER_READY.store(ready, Ordering::SeqCst);
}

/// Check if VFS server is ready
pub fn is_vfs_server_ready() -> bool {
    VFS_SERVER_READY.load(Ordering::SeqCst)
}

/// Get current process credentials (UID, GID)
fn get_current_credentials() -> (u32, u32) {
    // TODO: Get from current thread's credentials
    // For now, return debos user (UID 1000, GID 1000)
    (1000, 1000)
}

/// Convert kernel OpenFlags to VFS protocol flags
fn to_vfs_flags(flags: OpenFlags) -> u32 {
    let mut vfs_flags = 0u32;
    if flags.contains(OpenFlags::READ) {
        vfs_flags |= VfsOpenFlags::Read as u32;
    }
    if flags.contains(OpenFlags::WRITE) {
        vfs_flags |= VfsOpenFlags::Write as u32;
    }
    if flags.contains(OpenFlags::CREATE) {
        vfs_flags |= VfsOpenFlags::Create as u32;
    }
    if flags.contains(OpenFlags::APPEND) {
        vfs_flags |= VfsOpenFlags::Append as u32;
    }
    if flags.contains(OpenFlags::TRUNC) {
        vfs_flags |= VfsOpenFlags::Truncate as u32;
    }
    vfs_flags
}

/// Convert VFS error to kernel FsError
fn from_vfs_error(error: VfsError) -> FsError {
    match error {
        VfsError::Success => unreachable!(),
        VfsError::NotFound => FsError::NotFound,
        VfsError::AlreadyExists => FsError::AlreadyExists,
        VfsError::NotADirectory => FsError::NotADirectory,
        VfsError::IsADirectory => FsError::IsADirectory,
        VfsError::NotEmpty => FsError::NotEmpty,
        VfsError::PermissionDenied => FsError::PermissionDenied,
        VfsError::InvalidPath => FsError::InvalidPath,
        VfsError::NoSpace => FsError::NoSpace,
        VfsError::ReadOnly => FsError::ReadOnly,
        VfsError::IoError => FsError::IoError,
        VfsError::InvalidFd => FsError::InvalidFd,
        VfsError::TooManyOpenFiles => FsError::TooManyOpenFiles,
        VfsError::InvalidArgument => FsError::InvalidArgument,
        VfsError::InvalidFilesystem => FsError::InvalidFilesystem,
        VfsError::NoFilesystem => FsError::NoFilesystem,
        VfsError::NotSupported => FsError::NotSupported,
        VfsError::Unknown => FsError::IoError,
    }
}

/// Convert IPC error to kernel FsError
fn from_ipc_error(_error: IpcError) -> FsError {
    FsError::IoError
}

/// Convert VfsFileType to kernel InodeType
fn to_inode_type(file_type: u8) -> InodeType {
    match file_type {
        1 => InodeType::File,      // VfsFileType::Regular
        2 => InodeType::Directory, // VfsFileType::Directory
        3 => InodeType::Symlink,   // VfsFileType::Symlink
        _ => InodeType::File,
    }
}

/// Send a VFS request and receive response
fn vfs_call(request: &[u8]) -> FsResult<Vec<u8>> {
    let mut reply_buffer = [0u8; 4096];
    
    match crate::ipc::ipc_call(VFS_ENDPOINT, request, &mut reply_buffer) {
        Ok(reply_len) => {
            let response = reply_buffer[..reply_len].to_vec();
            
            // Parse response header
            if let Some(header) = VfsResponseHeader::from_bytes(&response) {
                if header.error == 0 {
                    Ok(response)
                } else {
                    Err(from_vfs_error(VfsError::from(header.error)))
                }
            } else {
                Err(FsError::IoError)
            }
        }
        Err(e) => Err(from_ipc_error(e)),
    }
}

// ========== Public VFS Client API ==========
// Note: These functions use u32 for fd to match the in-kernel VFS API

/// Open a file via VFS server
pub fn vfs_open(path: &str, flags: OpenFlags) -> FsResult<u32> {
    if !is_vfs_server_ready() {
        // Fall back to in-kernel VFS
        return super::vfs::open(path, flags);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Open, uid, gid);
    header.flags = to_vfs_flags(flags);
    header.path_len = path.len() as u32 + 1; // +1 for null terminator
    
    let request = build_request(&header, Some(path), None);
    let response = vfs_call(&request)?;
    
    // Parse response
    if let Some(resp_header) = VfsResponseHeader::from_bytes(&response) {
        Ok(resp_header.result as u32)
    } else {
        Err(FsError::IoError)
    }
}

/// Close a file via VFS server
pub fn vfs_close(fd: u32) -> FsResult<()> {
    if !is_vfs_server_ready() {
        return super::vfs::close(fd);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Close, uid, gid);
    header.fd = fd as i32;
    
    let request = build_request(&header, None, None);
    let _ = vfs_call(&request)?;
    
    Ok(())
}

/// Read from a file via VFS server
pub fn vfs_read(fd: u32, buffer: &mut [u8]) -> FsResult<usize> {
    if !is_vfs_server_ready() {
        return super::vfs::read(fd, buffer);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Read, uid, gid);
    header.fd = fd as i32;
    header.length = buffer.len() as u32;
    
    let request = build_request(&header, None, None);
    let response = vfs_call(&request)?;
    
    // Parse response and copy data
    if let Some(resp_header) = VfsResponseHeader::from_bytes(&response) {
        let data_start = VfsResponseHeader::SIZE;
        let data_len = resp_header.data_len as usize;
        
        if response.len() >= data_start + data_len {
            let actual_len = core::cmp::min(data_len, buffer.len());
            buffer[..actual_len].copy_from_slice(&response[data_start..data_start + actual_len]);
            Ok(actual_len)
        } else {
            Err(FsError::IoError)
        }
    } else {
        Err(FsError::IoError)
    }
}

/// Write to a file via VFS server
pub fn vfs_write(fd: u32, data: &[u8]) -> FsResult<usize> {
    if !is_vfs_server_ready() {
        return super::vfs::write(fd, data);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Write, uid, gid);
    header.fd = fd as i32;
    header.length = data.len() as u32;
    
    let request = build_request(&header, None, Some(data));
    let response = vfs_call(&request)?;
    
    if let Some(resp_header) = VfsResponseHeader::from_bytes(&response) {
        Ok(resp_header.result as usize)
    } else {
        Err(FsError::IoError)
    }
}

/// Get file stat via VFS server
pub fn vfs_stat(path: &str) -> FsResult<Stat> {
    if !is_vfs_server_ready() {
        return super::vfs::stat(path);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Stat, uid, gid);
    header.path_len = path.len() as u32 + 1;
    
    let request = build_request(&header, Some(path), None);
    let response = vfs_call(&request)?;
    
    // Parse VfsStat from response
    let stat_start = VfsResponseHeader::SIZE;
    if response.len() >= stat_start + VfsStat::SIZE {
        if let Some(vfs_stat) = VfsStat::from_bytes(&response[stat_start..]) {
            Ok(Stat {
                inode: vfs_stat.inode,
                inode_type: to_inode_type(vfs_stat.file_type),
                size: vfs_stat.size as usize,
                permissions: vfs_stat.permissions,
                uid: vfs_stat.uid,
                gid: vfs_stat.gid,
            })
        } else {
            Err(FsError::IoError)
        }
    } else {
        Err(FsError::IoError)
    }
}

/// Create a directory via VFS server
pub fn vfs_mkdir(path: &str) -> FsResult<()> {
    if !is_vfs_server_ready() {
        return super::vfs::mkdir(path);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Mkdir, uid, gid);
    header.path_len = path.len() as u32 + 1;
    header.flags = 0o755; // Default directory permissions
    
    let request = build_request(&header, Some(path), None);
    let _ = vfs_call(&request)?;
    
    Ok(())
}

/// Remove a directory via VFS server
pub fn vfs_rmdir(path: &str) -> FsResult<()> {
    if !is_vfs_server_ready() {
        return super::vfs::rmdir(path);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Rmdir, uid, gid);
    header.path_len = path.len() as u32 + 1;
    
    let request = build_request(&header, Some(path), None);
    let _ = vfs_call(&request)?;
    
    Ok(())
}

/// Remove a file via VFS server
pub fn vfs_unlink(path: &str) -> FsResult<()> {
    if !is_vfs_server_ready() {
        return super::vfs::unlink(path);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Unlink, uid, gid);
    header.path_len = path.len() as u32 + 1;
    
    let request = build_request(&header, Some(path), None);
    let _ = vfs_call(&request)?;
    
    Ok(())
}

/// Read directory entries via VFS server
pub fn vfs_readdir(path: &str) -> FsResult<Vec<DirEntry>> {
    if !is_vfs_server_ready() {
        return super::vfs::readdir(path);
    }
    
    let (uid, gid) = get_current_credentials();
    
    // First open the directory
    let mut open_header = VfsRequestHeader::new(VfsOp::Open, uid, gid);
    open_header.flags = VfsOpenFlags::Read as u32;
    open_header.path_len = path.len() as u32 + 1;
    
    let open_request = build_request(&open_header, Some(path), None);
    let open_response = vfs_call(&open_request)?;
    
    let fd = if let Some(resp_header) = VfsResponseHeader::from_bytes(&open_response) {
        resp_header.result
    } else {
        return Err(FsError::IoError);
    };
    
    // Now read directory
    let mut readdir_header = VfsRequestHeader::new(VfsOp::Readdir, uid, gid);
    readdir_header.fd = fd;
    
    let readdir_request = build_request(&readdir_header, None, None);
    let response = vfs_call(&readdir_request)?;
    
    // Close the directory
    let mut close_header = VfsRequestHeader::new(VfsOp::Close, uid, gid);
    close_header.fd = fd;
    let close_request = build_request(&close_header, None, None);
    let _ = vfs_call(&close_request);
    
    // Parse directory entries
    let mut entries = Vec::new();
    
    if let Some(resp_header) = VfsResponseHeader::from_bytes(&response) {
        let mut offset = VfsResponseHeader::SIZE;
        
        for _ in 0..resp_header.result {
            if offset + 11 > response.len() { // 8 (inode) + 1 (type) + 2 (name_len)
                break;
            }
            
            let inode = u64::from_le_bytes(response[offset..offset+8].try_into().unwrap_or([0;8]));
            let file_type = response[offset + 8];
            let name_len = u16::from_le_bytes(response[offset+9..offset+11].try_into().unwrap_or([0;2])) as usize;
            
            offset += 11;
            
            if offset + name_len > response.len() {
                break;
            }
            
            let name = String::from_utf8_lossy(&response[offset..offset+name_len]).into_owned();
            offset += name_len;
            
            entries.push(DirEntry {
                name,
                inode,
                inode_type: to_inode_type(file_type),
                size: 0, // Size not included in directory listing
            });
        }
    }
    
    Ok(entries)
}

/// Change working directory via VFS server
pub fn vfs_chdir(path: &str) -> FsResult<()> {
    if !is_vfs_server_ready() {
        return super::vfs::chdir(path);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Chdir, uid, gid);
    header.path_len = path.len() as u32 + 1;
    
    let request = build_request(&header, Some(path), None);
    let _ = vfs_call(&request)?;
    
    Ok(())
}

/// Get current working directory via VFS server
pub fn vfs_getcwd() -> FsResult<String> {
    if !is_vfs_server_ready() {
        return super::vfs::getcwd();
    }
    
    let (uid, gid) = get_current_credentials();
    
    let header = VfsRequestHeader::new(VfsOp::Getcwd, uid, gid);
    
    let request = build_request(&header, None, None);
    let response = vfs_call(&request)?;
    
    if let Some(resp_header) = VfsResponseHeader::from_bytes(&response) {
        let data_start = VfsResponseHeader::SIZE;
        let data_len = resp_header.data_len as usize;
        
        if response.len() >= data_start + data_len {
            let cwd = String::from_utf8_lossy(&response[data_start..data_start + data_len]).into_owned();
            Ok(cwd)
        } else {
            Err(FsError::IoError)
        }
    } else {
        Err(FsError::IoError)
    }
}

/// Create/touch a file via VFS server
pub fn vfs_touch(path: &str) -> FsResult<()> {
    if !is_vfs_server_ready() {
        return super::vfs::touch(path);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Touch, uid, gid);
    header.path_len = path.len() as u32 + 1;
    
    let request = build_request(&header, Some(path), None);
    let _ = vfs_call(&request)?;
    
    Ok(())
}

/// Seek in a file via VFS server
pub fn vfs_seek(fd: u32, pos: SeekFrom) -> FsResult<u64> {
    if !is_vfs_server_ready() {
        // Fall back to in-kernel VFS
        return super::vfs::seek(fd, pos);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Seek, uid, gid);
    header.fd = fd as i32;
    
    match pos {
        SeekFrom::Start(offset) => {
            header.flags = VfsSeekFrom::Start as u32;
            header.offset = offset;
        }
        SeekFrom::Current(offset) => {
            header.flags = VfsSeekFrom::Current as u32;
            header.offset = offset as u64;
        }
        SeekFrom::End(offset) => {
            header.flags = VfsSeekFrom::End as u32;
            header.offset = offset as u64;
        }
    }
    
    let request = build_request(&header, None, None);
    let response = vfs_call(&request)?;
    
    if let Some(resp_header) = VfsResponseHeader::from_bytes(&response) {
        Ok(resp_header.result as u64)
    } else {
        Err(FsError::IoError)
    }
}

/// Change file mode via VFS server
pub fn vfs_chmod(path: &str, mode: u16) -> FsResult<()> {
    if !is_vfs_server_ready() {
        return Err(FsError::NotSupported);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Chmod, uid, gid);
    header.path_len = path.len() as u32 + 1;
    header.flags = mode as u32;
    
    let request = build_request(&header, Some(path), None);
    let _ = vfs_call(&request)?;
    
    Ok(())
}

/// Change file owner via VFS server
pub fn vfs_chown(path: &str, owner: u32, group: u32) -> FsResult<()> {
    if !is_vfs_server_ready() {
        return Err(FsError::NotSupported);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Chown, uid, gid);
    header.path_len = path.len() as u32 + 1;
    header.flags = owner;
    header.length = group;
    
    let request = build_request(&header, Some(path), None);
    let _ = vfs_call(&request)?;
    
    Ok(())
}

/// Truncate a file via VFS server  
pub fn vfs_truncate(fd: u32, size: u64) -> FsResult<()> {
    if !is_vfs_server_ready() {
        return Err(FsError::NotSupported);
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Truncate, uid, gid);
    header.fd = fd as i32;
    header.offset = size;
    
    let request = build_request(&header, None, None);
    let _ = vfs_call(&request)?;
    
    Ok(())
}

/// Sync file to storage via VFS server
pub fn vfs_sync(fd: u32) -> FsResult<()> {
    if !is_vfs_server_ready() {
        return Ok(()); // No-op for in-kernel RamFS
    }
    
    let (uid, gid) = get_current_credentials();
    
    let mut header = VfsRequestHeader::new(VfsOp::Sync, uid, gid);
    header.fd = fd as i32;
    
    let request = build_request(&header, None, None);
    let _ = vfs_call(&request)?;
    
    Ok(())
}
