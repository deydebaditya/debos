//! DebOS Filesystem Module
//!
//! Provides in-kernel filesystem support for early boot and shell operations.
//! This is a temporary solution until the full VFS server is implemented.

pub mod path;
pub mod ramfs;
pub mod vfs;
pub mod fat32;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

// Re-export commonly used types
pub use path::{normalize_path, join_path};
pub use vfs::{open, close, read, write, mkdir, rmdir, unlink, stat, readdir, chdir, getcwd, touch, read_to_string, write_string};

/// Inode types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeType {
    File,
    Directory,
    Symlink,
}

/// File open flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: u32 {
        const READ   = 0b00001;
        const WRITE  = 0b00010;
        const CREATE = 0b00100;
        const APPEND = 0b01000;
        const TRUNC  = 0b10000;
    }
}

/// Seek position
#[derive(Debug, Clone, Copy)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

/// File/directory metadata
#[derive(Debug, Clone)]
pub struct Stat {
    pub inode: u64,
    pub inode_type: InodeType,
    pub size: usize,
    pub permissions: u16,
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub inode: u64,
    pub inode_type: InodeType,
    pub size: usize,
}

/// Filesystem error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    /// File or directory not found
    NotFound,
    /// File or directory already exists
    AlreadyExists,
    /// Expected directory, got file
    NotADirectory,
    /// Expected file, got directory
    IsADirectory,
    /// Directory not empty
    NotEmpty,
    /// Permission denied
    PermissionDenied,
    /// Invalid path
    InvalidPath,
    /// No space left
    NoSpace,
    /// Read-only filesystem
    ReadOnly,
    /// Generic I/O error
    IoError,
    /// Invalid file descriptor
    InvalidFd,
    /// Too many open files
    TooManyOpenFiles,
    /// Invalid argument
    InvalidArgument,
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsError::NotFound => write!(f, "No such file or directory"),
            FsError::AlreadyExists => write!(f, "File exists"),
            FsError::NotADirectory => write!(f, "Not a directory"),
            FsError::IsADirectory => write!(f, "Is a directory"),
            FsError::NotEmpty => write!(f, "Directory not empty"),
            FsError::PermissionDenied => write!(f, "Permission denied"),
            FsError::InvalidPath => write!(f, "Invalid path"),
            FsError::NoSpace => write!(f, "No space left on device"),
            FsError::ReadOnly => write!(f, "Read-only file system"),
            FsError::IoError => write!(f, "I/O error"),
            FsError::InvalidFd => write!(f, "Bad file descriptor"),
            FsError::TooManyOpenFiles => write!(f, "Too many open files"),
            FsError::InvalidArgument => write!(f, "Invalid argument"),
        }
    }
}

/// Result type for filesystem operations
pub type FsResult<T> = Result<T, FsError>;

/// Initialize the filesystem
pub fn init() {
    vfs::init();
    crate::println!("  Filesystem initialized (RamFS)");
}

