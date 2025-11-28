//! VFS IPC Protocol
//!
//! Defines the message format for communication between VFS clients
//! and the VFS server via IPC.
//!
//! This protocol enables filesystem operations to be handled in userspace
//! while maintaining a clean interface for both kernel and user applications.

use alloc::string::String;
use alloc::vec::Vec;

/// Maximum path length in bytes
pub const MAX_PATH_LEN: usize = 1024;

/// Maximum filename length in bytes
pub const MAX_NAME_LEN: usize = 256;

/// Maximum data payload per message
pub const MAX_DATA_LEN: usize = 3072;

/// VFS operation codes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsOp {
    /// Open a file or directory
    Open = 1,
    /// Close a file handle
    Close = 2,
    /// Read data from file
    Read = 3,
    /// Write data to file
    Write = 4,
    /// Get file/directory metadata
    Stat = 5,
    /// Create a directory
    Mkdir = 6,
    /// Remove a directory
    Rmdir = 7,
    /// Remove a file
    Unlink = 8,
    /// List directory contents
    Readdir = 9,
    /// Seek to position in file
    Seek = 10,
    /// Sync file to storage
    Sync = 11,
    /// Rename file/directory
    Rename = 12,
    /// Create symlink
    Symlink = 13,
    /// Read symlink target
    Readlink = 14,
    /// Truncate file
    Truncate = 15,
    /// Change file permissions
    Chmod = 16,
    /// Change file owner
    Chown = 17,
    /// Create a regular file
    Touch = 18,
    /// Change current working directory (per-process)
    Chdir = 19,
    /// Get current working directory
    Getcwd = 20,
    /// Mount a filesystem
    Mount = 100,
    /// Unmount a filesystem
    Unmount = 101,
    /// Invalid/unknown operation
    Invalid = 255,
}

impl From<u8> for VfsOp {
    fn from(v: u8) -> Self {
        match v {
            1 => VfsOp::Open,
            2 => VfsOp::Close,
            3 => VfsOp::Read,
            4 => VfsOp::Write,
            5 => VfsOp::Stat,
            6 => VfsOp::Mkdir,
            7 => VfsOp::Rmdir,
            8 => VfsOp::Unlink,
            9 => VfsOp::Readdir,
            10 => VfsOp::Seek,
            11 => VfsOp::Sync,
            12 => VfsOp::Rename,
            13 => VfsOp::Symlink,
            14 => VfsOp::Readlink,
            15 => VfsOp::Truncate,
            16 => VfsOp::Chmod,
            17 => VfsOp::Chown,
            18 => VfsOp::Touch,
            19 => VfsOp::Chdir,
            20 => VfsOp::Getcwd,
            100 => VfsOp::Mount,
            101 => VfsOp::Unmount,
            _ => VfsOp::Invalid,
        }
    }
}

/// Open flags (matches kernel's OpenFlags)
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum VfsOpenFlags {
    Read = 0x01,
    Write = 0x02,
    Create = 0x04,
    Append = 0x08,
    Truncate = 0x10,
}

/// Seek origin
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum VfsSeekFrom {
    Start = 0,
    Current = 1,
    End = 2,
}

/// File type in VFS
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsFileType {
    Regular = 1,
    Directory = 2,
    Symlink = 3,
}

/// VFS error codes
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

impl From<i32> for VfsError {
    fn from(v: i32) -> Self {
        match v {
            0 => VfsError::Success,
            -1 => VfsError::NotFound,
            -2 => VfsError::AlreadyExists,
            -3 => VfsError::NotADirectory,
            -4 => VfsError::IsADirectory,
            -5 => VfsError::NotEmpty,
            -6 => VfsError::PermissionDenied,
            -7 => VfsError::InvalidPath,
            -8 => VfsError::NoSpace,
            -9 => VfsError::ReadOnly,
            -10 => VfsError::IoError,
            -11 => VfsError::InvalidFd,
            -12 => VfsError::TooManyOpenFiles,
            -13 => VfsError::InvalidArgument,
            -14 => VfsError::InvalidFilesystem,
            -15 => VfsError::NoFilesystem,
            -16 => VfsError::NotSupported,
            _ => VfsError::Unknown,
        }
    }
}

/// VFS Request header (always at start of message)
/// 
/// Layout:
/// - Byte 0: Operation code (VfsOp)
/// - Bytes 1-4: Request ID (for tracking)
/// - Bytes 5-8: UID of requesting process
/// - Bytes 9-12: GID of requesting process
/// - Bytes 13-16: File descriptor (for operations that need it)
/// - Bytes 17-20: Flags/options
/// - Bytes 21-28: Offset (for seek/read/write)
/// - Bytes 29-32: Length (for read/write)
/// - Bytes 33-36: Path length
/// - Bytes 37+: Path data (null-terminated)
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

    /// Create a new request header
    pub fn new(op: VfsOp, uid: u32, gid: u32) -> Self {
        static REQUEST_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(1);
        
        Self {
            op: op as u8,
            request_id: REQUEST_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed),
            uid,
            gid,
            fd: -1,
            flags: 0,
            offset: 0,
            length: 0,
            path_len: 0,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        unsafe { core::mem::transmute_copy(self) }
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let mut header = [0u8; Self::SIZE];
        header.copy_from_slice(&bytes[..Self::SIZE]);
        Some(unsafe { core::mem::transmute(header) })
    }
}

/// VFS Response header
/// 
/// Layout:
/// - Bytes 0-3: Request ID (matches request)
/// - Bytes 4-7: Error code (VfsError as i32, 0 = success)
/// - Bytes 8-11: Result value (e.g., bytes read, fd returned)
/// - Bytes 12-15: Data length (for variable-length responses)
/// - Bytes 16+: Response data
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

    /// Create a success response
    pub fn success(request_id: u32, result: i32) -> Self {
        Self {
            request_id,
            error: VfsError::Success as i32,
            result,
            data_len: 0,
        }
    }

    /// Create a success response with data
    pub fn success_with_data(request_id: u32, result: i32, data_len: u32) -> Self {
        Self {
            request_id,
            error: VfsError::Success as i32,
            result,
            data_len,
        }
    }

    /// Create an error response
    pub fn error(request_id: u32, error: VfsError) -> Self {
        Self {
            request_id,
            error: error as i32,
            result: -1,
            data_len: 0,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        unsafe { core::mem::transmute_copy(self) }
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let mut header = [0u8; Self::SIZE];
        header.copy_from_slice(&bytes[..Self::SIZE]);
        Some(unsafe { core::mem::transmute(header) })
    }
}

/// File stat information (returned by STAT operation)
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

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        unsafe { core::mem::transmute_copy(self) }
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let mut stat = [0u8; Self::SIZE];
        stat.copy_from_slice(&bytes[..Self::SIZE]);
        Some(unsafe { core::mem::transmute(stat) })
    }
}

/// Directory entry (returned by READDIR operation)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VfsDirEntry {
    pub inode: u64,
    pub file_type: u8,
    pub name_len: u16,
    // Followed by name bytes
}

impl VfsDirEntry {
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

/// Well-known VFS endpoint ID
/// 
/// The VFS server registers on this endpoint ID when it starts.
/// All filesystem operations should be sent to this endpoint.
pub const VFS_ENDPOINT_ID: u64 = 1000;

/// Build a VFS request message
pub fn build_request(header: &VfsRequestHeader, path: Option<&str>, data: Option<&[u8]>) -> Vec<u8> {
    let mut msg = Vec::with_capacity(
        VfsRequestHeader::SIZE + 
        path.map(|p| p.len() + 1).unwrap_or(0) +
        data.map(|d| d.len()).unwrap_or(0)
    );
    
    // Write header
    msg.extend_from_slice(&header.to_bytes());
    
    // Write path if present
    if let Some(p) = path {
        msg.extend_from_slice(p.as_bytes());
        msg.push(0); // null terminator
    }
    
    // Write data if present
    if let Some(d) = data {
        msg.extend_from_slice(d);
    }
    
    msg
}

/// Parse path from request message
pub fn parse_path(msg: &[u8], header: &VfsRequestHeader) -> Option<String> {
    if header.path_len == 0 {
        return None;
    }
    
    let path_start = VfsRequestHeader::SIZE;
    let path_end = path_start + header.path_len as usize;
    
    if msg.len() < path_end {
        return None;
    }
    
    let path_bytes = &msg[path_start..path_end];
    // Remove null terminator if present
    let path_bytes = if path_bytes.last() == Some(&0) {
        &path_bytes[..path_bytes.len() - 1]
    } else {
        path_bytes
    };
    
    String::from_utf8(path_bytes.to_vec()).ok()
}

/// Parse data from request message
pub fn parse_data<'a>(msg: &'a [u8], header: &VfsRequestHeader) -> Option<&'a [u8]> {
    let data_start = VfsRequestHeader::SIZE + header.path_len as usize;
    let data_len = header.length as usize;
    
    if data_len == 0 || msg.len() < data_start + data_len {
        return None;
    }
    
    Some(&msg[data_start..data_start + data_len])
}

