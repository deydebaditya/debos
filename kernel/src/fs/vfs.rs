//! Virtual Filesystem Layer
//!
//! Provides the high-level filesystem API with file handles,
//! working directory tracking, and thread-local state.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

use super::{OpenFlags, SeekFrom, FsError, FsResult, Stat, DirEntry, InodeType};
use super::ramfs::RamFs;
use super::path;

/// Get current user's uid/gid for file ownership
/// This is called BEFORE acquiring VFS lock to avoid nested locks
fn get_current_owner() -> (u32, u32) {
    // FIXME: Temporarily hardcode to avoid potential deadlock with scheduler lock
    // The scheduler lock might be held when this is called, causing a deadlock
    // TODO: Fix this properly by ensuring scheduler lock is not held when calling VFS operations
    // For now, use default debos user (UID 1000) to unblock the shell
    (1000, 1000)
    
    // Original code (commented out to debug deadlock):
    // match crate::scheduler::current_credentials() {
    //     Some(creds) => (creds.uid.as_raw(), creds.gid.as_raw()),
    //     None => (0, 0),
    // }
}

/// Maximum number of open files per process
const MAX_OPEN_FILES: usize = 256;

/// File handle representing an open file
#[derive(Debug)]
struct FileHandle {
    /// Inode ID of the file
    inode_id: u64,
    /// Current position in file
    position: usize,
    /// Open flags
    flags: OpenFlags,
}

/// Global VFS state
struct VfsState {
    /// The underlying filesystem
    fs: RamFs,
    /// Open file handles (fd -> handle)
    handles: BTreeMap<u32, FileHandle>,
    /// Next file descriptor to allocate
    next_fd: AtomicU32,
    /// Current working directory
    cwd: String,
}

/// Global VFS instance
static VFS: Mutex<Option<VfsState>> = Mutex::new(None);

/// Initialize the VFS
pub fn init() {
    let mut vfs = VFS.lock();
    
    let mut state = VfsState {
        fs: RamFs::new(),
        handles: BTreeMap::new(),
        next_fd: AtomicU32::new(3), // 0, 1, 2 reserved for stdin/stdout/stderr
        cwd: String::from("/"),
    };
    
    // Create some default directories (as root)
    let _ = state.fs.create_dir("/", "home", 0, 0);
    let _ = state.fs.create_dir("/", "tmp", 0, 0);
    let _ = state.fs.create_dir("/", "etc", 0, 0);
    let _ = state.fs.create_dir("/", "var", 0, 0);
    
    *vfs = Some(state);
}

/// Resolve a path relative to CWD
fn resolve_path(vfs: &VfsState, p: &str) -> String {
    if path::is_absolute(p) {
        path::normalize_path(p)
    } else {
        path::join_path(&vfs.cwd, p)
    }
}

/// Open a file
pub fn open(file_path: &str, flags: OpenFlags) -> FsResult<u32> {
    // Get credentials BEFORE acquiring VFS lock if we might create a file
    let (uid, gid) = if flags.contains(OpenFlags::CREATE) {
        get_current_owner()
    } else {
        (0, 0) // Won't be used
    };
    
    let mut vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_mut().ok_or(FsError::IoError)?;
    
    let resolved = resolve_path(vfs, file_path);
    
    // Try to find the file
    let inode_id = match vfs.fs.lookup(&resolved) {
        Ok(id) => {
            // File exists
            let inode = vfs.fs.get(id)?;
            
            if inode.inode_type == InodeType::Directory {
                return Err(FsError::IsADirectory);
            }
            
            // If TRUNC flag, truncate the file
            if flags.contains(OpenFlags::TRUNC) && flags.contains(OpenFlags::WRITE) {
                vfs.fs.truncate(id, 0)?;
            }
            
            id
        }
        Err(FsError::NotFound) if flags.contains(OpenFlags::CREATE) => {
            // Create the file
            let (parent_path, filename) = path::split(&resolved);
            if filename.is_empty() {
                return Err(FsError::InvalidPath);
            }
            vfs.fs.create_file(&parent_path, &filename, uid, gid)?
        }
        Err(e) => return Err(e),
    };
    
    // Allocate file descriptor
    let fd = vfs.next_fd.fetch_add(1, Ordering::SeqCst);
    
    if vfs.handles.len() >= MAX_OPEN_FILES {
        return Err(FsError::TooManyOpenFiles);
    }
    
    let position = if flags.contains(OpenFlags::APPEND) {
        vfs.fs.get(inode_id)?.size
    } else {
        0
    };
    
    vfs.handles.insert(fd, FileHandle {
        inode_id,
        position,
        flags,
    });
    
    Ok(fd)
}

/// Close a file
pub fn close(fd: u32) -> FsResult<()> {
    let mut vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_mut().ok_or(FsError::IoError)?;
    
    vfs.handles.remove(&fd).ok_or(FsError::InvalidFd)?;
    Ok(())
}

/// Read from an open file
pub fn read(fd: u32, buf: &mut [u8]) -> FsResult<usize> {
    let mut vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_mut().ok_or(FsError::IoError)?;
    
    let handle = vfs.handles.get(&fd).ok_or(FsError::InvalidFd)?;
    
    if !handle.flags.contains(OpenFlags::READ) {
        return Err(FsError::PermissionDenied);
    }
    
    let inode_id = handle.inode_id;
    let position = handle.position;
    
    let bytes_read = vfs.fs.read(inode_id, position, buf)?;
    
    // Update position
    if let Some(h) = vfs.handles.get_mut(&fd) {
        h.position += bytes_read;
    }
    
    Ok(bytes_read)
}

/// Write to an open file
pub fn write(fd: u32, data: &[u8]) -> FsResult<usize> {
    let mut vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_mut().ok_or(FsError::IoError)?;
    
    let handle = vfs.handles.get(&fd).ok_or(FsError::InvalidFd)?;
    
    if !handle.flags.contains(OpenFlags::WRITE) {
        return Err(FsError::PermissionDenied);
    }
    
    let inode_id = handle.inode_id;
    let position = handle.position;
    
    let bytes_written = vfs.fs.write(inode_id, position, data)?;
    
    // Update position
    if let Some(h) = vfs.handles.get_mut(&fd) {
        h.position += bytes_written;
    }
    
    Ok(bytes_written)
}

/// Seek in an open file
pub fn seek(fd: u32, pos: SeekFrom) -> FsResult<u64> {
    let mut vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_mut().ok_or(FsError::IoError)?;
    
    let handle = vfs.handles.get(&fd).ok_or(FsError::InvalidFd)?;
    let inode = vfs.fs.get(handle.inode_id)?;
    
    let new_pos = match pos {
        SeekFrom::Start(offset) => offset as usize,
        SeekFrom::End(offset) => {
            if offset < 0 {
                inode.size.saturating_sub((-offset) as usize)
            } else {
                inode.size + offset as usize
            }
        }
        SeekFrom::Current(offset) => {
            if offset < 0 {
                handle.position.saturating_sub((-offset) as usize)
            } else {
                handle.position + offset as usize
            }
        }
    };
    
    if let Some(h) = vfs.handles.get_mut(&fd) {
        h.position = new_pos;
    }
    
    Ok(new_pos as u64)
}

/// Create a directory
pub fn mkdir(dir_path: &str) -> FsResult<()> {
    // Get credentials BEFORE acquiring VFS lock to avoid nested locks
    let (uid, gid) = get_current_owner();
    
    let mut vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_mut().ok_or(FsError::IoError)?;
    
    let resolved = resolve_path(vfs, dir_path);
    let (parent_path, dirname) = path::split(&resolved);
    
    if dirname.is_empty() {
        return Err(FsError::InvalidPath);
    }
    
    vfs.fs.create_dir(&parent_path, &dirname, uid, gid)?;
    Ok(())
}

/// Remove an empty directory
pub fn rmdir(dir_path: &str) -> FsResult<()> {
    let mut vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_mut().ok_or(FsError::IoError)?;
    
    let resolved = resolve_path(vfs, dir_path);
    vfs.fs.rmdir(&resolved)
}

/// Remove a file
pub fn unlink(file_path: &str) -> FsResult<()> {
    let mut vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_mut().ok_or(FsError::IoError)?;
    
    let resolved = resolve_path(vfs, file_path);
    vfs.fs.unlink(&resolved)
}

/// Get file/directory metadata
pub fn stat(file_path: &str) -> FsResult<Stat> {
    let vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_ref().ok_or(FsError::IoError)?;
    
    let resolved = resolve_path(vfs, file_path);
    vfs.fs.stat(&resolved)
}

/// Read directory contents
pub fn readdir(dir_path: &str) -> FsResult<Vec<DirEntry>> {
    let vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_ref().ok_or(FsError::IoError)?;
    
    let resolved = resolve_path(vfs, dir_path);
    let inode_id = vfs.fs.lookup(&resolved)?;
    vfs.fs.readdir(inode_id)
}

/// Change current working directory
pub fn chdir(dir_path: &str) -> FsResult<()> {
    let mut vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_mut().ok_or(FsError::IoError)?;
    
    let resolved = resolve_path(vfs, dir_path);
    
    // Verify it exists and is a directory
    let inode_id = vfs.fs.lookup(&resolved)?;
    let inode = vfs.fs.get(inode_id)?;
    
    if inode.inode_type != InodeType::Directory {
        return Err(FsError::NotADirectory);
    }
    
    vfs.cwd = resolved;
    Ok(())
}

/// Get current working directory
pub fn getcwd() -> FsResult<String> {
    let vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_ref().ok_or(FsError::IoError)?;
    
    Ok(vfs.cwd.clone())
}

/// Create an empty file or update timestamp
pub fn touch(file_path: &str) -> FsResult<()> {
    // Get credentials BEFORE acquiring VFS lock
    let (uid, gid) = get_current_owner();
    
    let mut vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_mut().ok_or(FsError::IoError)?;
    
    let resolved = resolve_path(vfs, file_path);
    
    // Check if file exists
    match vfs.fs.lookup(&resolved) {
        Ok(_) => {
            // File exists, would update timestamp (not implemented)
            Ok(())
        }
        Err(FsError::NotFound) => {
            // Create new file
            let (parent_path, filename) = path::split(&resolved);
            if filename.is_empty() {
                return Err(FsError::InvalidPath);
            }
            vfs.fs.create_file(&parent_path, &filename, uid, gid)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Read entire file to string
pub fn read_to_string(file_path: &str) -> FsResult<String> {
    let vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_ref().ok_or(FsError::IoError)?;
    
    let resolved = resolve_path(vfs, file_path);
    let inode_id = vfs.fs.lookup(&resolved)?;
    let inode = vfs.fs.get(inode_id)?;
    
    if inode.inode_type == InodeType::Directory {
        return Err(FsError::IsADirectory);
    }
    
    // Read all data
    let mut buf = alloc::vec![0u8; inode.size];
    vfs.fs.read(inode_id, 0, &mut buf)?;
    
    String::from_utf8(buf).map_err(|_| FsError::IoError)
}

/// Write string to file (creates or overwrites)
pub fn write_string(file_path: &str, content: &str) -> FsResult<()> {
    // Get credentials BEFORE acquiring VFS lock
    let (uid, gid) = get_current_owner();
    
    let mut vfs_guard = VFS.lock();
    let vfs = vfs_guard.as_mut().ok_or(FsError::IoError)?;
    
    let resolved = resolve_path(vfs, file_path);
    
    // Get or create file
    let inode_id = match vfs.fs.lookup(&resolved) {
        Ok(id) => {
            let inode = vfs.fs.get(id)?;
            if inode.inode_type == InodeType::Directory {
                return Err(FsError::IsADirectory);
            }
            // Truncate existing file
            vfs.fs.truncate(id, 0)?;
            id
        }
        Err(FsError::NotFound) => {
            let (parent_path, filename) = path::split(&resolved);
            if filename.is_empty() {
                return Err(FsError::InvalidPath);
            }
            vfs.fs.create_file(&parent_path, &filename, uid, gid)?
        }
        Err(e) => return Err(e),
    };
    
    // Write content
    vfs.fs.write(inode_id, 0, content.as_bytes())?;
    
    Ok(())
}

