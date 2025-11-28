# DebOS Filesystem Implementation Plan

> **Status:** Not Implemented (Stubs Only)  
> **Priority:** Phase 2 - Core Drivers  
> **Estimated Effort:** 4-6 weeks

---

## Table of Contents

1. [Current State](#current-state)
2. [Architecture Overview](#architecture-overview)
3. [Implementation Phases](#implementation-phases)
4. [Component Details](#component-details)
5. [Shell Commands](#shell-commands)
6. [API Reference](#api-reference)
7. [Testing Strategy](#testing-strategy)

---

## Current State

### What Exists Today

| Component | Status | Location |
|-----------|--------|----------|
| VFS Server | Stub only | `servers/vfs/src/main.rs` |
| libdebos fs module | Stub only | `libdebos/src/fs.rs` |
| Shell fs commands | Not implemented | `kernel/src/shell/commands.rs` |
| Block device drivers | Stubs only | `drivers/virtio_block/` |

### What's Missing

- ❌ In-memory filesystem (ramfs/tmpfs)
- ❌ VFS abstraction layer
- ❌ File descriptors and handles
- ❌ Directory operations
- ❌ Path resolution
- ❌ Block device interface
- ❌ FAT32/ext4 filesystem drivers
- ❌ Shell commands (ls, cd, mkdir, cat, etc.)

---

## Architecture Overview

### DebOS Filesystem Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      User Applications                           │
│                  (use libdebos filesystem API)                   │
├─────────────────────────────────────────────────────────────────┤
│                         libdebos                                 │
│              fs::open(), fs::read(), fs::mkdir()                 │
│                    (IPC to VFS Server)                           │
├─────────────────────────────────────────────────────────────────┤
│                       VFS Server                                 │
│    ┌─────────────┬─────────────┬─────────────┬───────────┐      │
│    │   Path      │   File      │   Mount     │  Inode    │      │
│    │  Resolver   │ Descriptors │   Table     │  Cache    │      │
│    └─────────────┴─────────────┴─────────────┴───────────┘      │
│                            │                                     │
│    ┌─────────────┬─────────────┬─────────────┬───────────┐      │
│    │   RamFS     │   FAT32     │   ext4      │  DebFS    │      │
│    │  (memory)   │  (driver)   │  (driver)   │ (native)  │      │
│    └─────────────┴─────────────┴─────────────┴───────────┘      │
├─────────────────────────────────────────────────────────────────┤
│                     Block Device Layer                           │
│           (IPC to block device drivers)                          │
├─────────────────────────────────────────────────────────────────┤
│              VirtIO-Block │ NVMe │ RAM Disk                      │
│                    (userspace drivers)                           │
├─────────────────────────────────────────────────────────────────┤
│                   DeK (Kernel) - IPC & Memory                    │
└─────────────────────────────────────────────────────────────────┘
```

### Design Principles

1. **Microkernel Design**: VFS runs in userspace, communicates via IPC
2. **POSIX-like API**: Familiar interface (open, read, write, close)
3. **Modular Filesystems**: Pluggable FS drivers (ramfs, FAT32, ext4)
4. **In-Kernel Bootstrap**: Initial ramfs in kernel for early boot

---

## Implementation Phases

### Phase 2A: In-Kernel RamFS (Week 1-2)

Start with a simple in-memory filesystem in the kernel for bootstrapping.

#### Goals
- Basic file/directory operations
- Shell commands working
- No external dependencies (no block devices yet)

#### Deliverables
- [x] `kernel/src/fs/mod.rs` - Core filesystem module
- [x] `kernel/src/fs/ramfs.rs` - RAM-based filesystem
- [x] `kernel/src/fs/vfs.rs` - Virtual filesystem layer
- [x] `kernel/src/fs/path.rs` - Path parsing and resolution
- [x] Shell commands: `ls`, `cd`, `pwd`, `mkdir`, `touch`, `cat`, `rm`

### Phase 2B: VFS Server (Week 3-4)

Move filesystem to userspace server.

#### Goals
- VFS server running as userspace process
- IPC protocol for filesystem operations
- Mount/unmount support

#### Deliverables
- [ ] `servers/vfs/src/main.rs` - VFS server implementation
- [ ] `servers/vfs/src/protocol.rs` - IPC message definitions
- [ ] `servers/vfs/src/mount.rs` - Mount table management
- [ ] `libdebos/src/fs.rs` - Client-side filesystem API

### Phase 2C: Block Device & FAT32 (Week 5-6)

Add persistent storage support.

#### Goals
- Block device abstraction
- FAT32 read/write support
- Disk image mounting

#### Deliverables
- [ ] `drivers/virtio_block/` - VirtIO block driver
- [ ] `servers/vfs/src/fat32.rs` - FAT32 filesystem driver
- [ ] `servers/vfs/src/block.rs` - Block device interface

---

## Component Details

### 1. In-Kernel RamFS Structure

```rust
// kernel/src/fs/mod.rs

/// Inode types
pub enum InodeType {
    File,
    Directory,
    Symlink,
}

/// Inode - represents a file or directory
pub struct Inode {
    pub id: u64,
    pub inode_type: InodeType,
    pub name: String,
    pub size: usize,
    pub permissions: u16,
    pub created: u64,
    pub modified: u64,
    pub data: Option<Vec<u8>>,          // For files
    pub children: Option<Vec<u64>>,      // For directories (inode IDs)
    pub parent: Option<u64>,
}

/// File handle for open files
pub struct FileHandle {
    pub inode_id: u64,
    pub position: usize,
    pub flags: OpenFlags,
}

/// Open flags
bitflags! {
    pub struct OpenFlags: u32 {
        const READ    = 0b0001;
        const WRITE   = 0b0010;
        const CREATE  = 0b0100;
        const APPEND  = 0b1000;
        const TRUNC   = 0b10000;
    }
}
```

### 2. RamFS Implementation

```rust
// kernel/src/fs/ramfs.rs

pub struct RamFs {
    inodes: BTreeMap<u64, Inode>,
    next_inode_id: AtomicU64,
    root_inode: u64,
}

impl RamFs {
    /// Create a new RamFS with root directory
    pub fn new() -> Self;
    
    /// Create a file or directory
    pub fn create(&mut self, parent: u64, name: &str, inode_type: InodeType) -> Result<u64, FsError>;
    
    /// Look up an inode by path
    pub fn lookup(&self, path: &str) -> Result<u64, FsError>;
    
    /// Read file contents
    pub fn read(&self, inode: u64, offset: usize, buf: &mut [u8]) -> Result<usize, FsError>;
    
    /// Write to file
    pub fn write(&mut self, inode: u64, offset: usize, data: &[u8]) -> Result<usize, FsError>;
    
    /// List directory contents
    pub fn readdir(&self, inode: u64) -> Result<Vec<DirEntry>, FsError>;
    
    /// Remove file or directory
    pub fn remove(&mut self, path: &str) -> Result<(), FsError>;
    
    /// Get inode metadata
    pub fn stat(&self, inode: u64) -> Result<Stat, FsError>;
}
```

### 3. VFS Layer

```rust
// kernel/src/fs/vfs.rs

/// Global filesystem state
pub struct Vfs {
    /// The mounted filesystem (initially RamFS)
    fs: RamFs,
    /// Open file handles per thread
    handles: BTreeMap<ThreadId, BTreeMap<u32, FileHandle>>,
    /// Next file descriptor number
    next_fd: AtomicU32,
    /// Current working directory per thread
    cwd: BTreeMap<ThreadId, String>,
}

impl Vfs {
    // POSIX-like operations
    pub fn open(&mut self, path: &str, flags: OpenFlags) -> Result<u32, FsError>;
    pub fn close(&mut self, fd: u32) -> Result<(), FsError>;
    pub fn read(&mut self, fd: u32, buf: &mut [u8]) -> Result<usize, FsError>;
    pub fn write(&mut self, fd: u32, data: &[u8]) -> Result<usize, FsError>;
    pub fn seek(&mut self, fd: u32, offset: i64, whence: SeekFrom) -> Result<u64, FsError>;
    pub fn mkdir(&mut self, path: &str) -> Result<(), FsError>;
    pub fn rmdir(&mut self, path: &str) -> Result<(), FsError>;
    pub fn unlink(&mut self, path: &str) -> Result<(), FsError>;
    pub fn stat(&self, path: &str) -> Result<Stat, FsError>;
    pub fn readdir(&self, path: &str) -> Result<Vec<DirEntry>, FsError>;
    pub fn chdir(&mut self, path: &str) -> Result<(), FsError>;
    pub fn getcwd(&self) -> Result<String, FsError>;
}
```

### 4. Path Resolution

```rust
// kernel/src/fs/path.rs

/// Normalize and resolve a path
pub fn normalize(path: &str) -> String {
    // Handle:
    // - Relative vs absolute paths
    // - . and .. components
    // - Multiple slashes
    // - Trailing slashes
}

/// Split path into components
pub fn components(path: &str) -> Vec<&str>;

/// Join path components
pub fn join(base: &str, path: &str) -> String;

/// Get parent directory
pub fn parent(path: &str) -> Option<&str>;

/// Get filename from path
pub fn filename(path: &str) -> Option<&str>;
```

---

## Shell Commands

### Command Implementation Plan

| Command | Description | Priority | Complexity |
|---------|-------------|----------|------------|
| `pwd` | Print working directory | P0 | Low |
| `ls` | List directory contents | P0 | Medium |
| `cd` | Change directory | P0 | Low |
| `mkdir` | Create directory | P0 | Low |
| `touch` | Create empty file | P0 | Low |
| `cat` | Display file contents | P0 | Low |
| `echo > file` | Write to file | P1 | Medium |
| `rm` | Remove file | P1 | Low |
| `rmdir` | Remove directory | P1 | Low |
| `cp` | Copy file | P2 | Medium |
| `mv` | Move/rename file | P2 | Medium |
| `find` | Search for files | P3 | High |
| `grep` | Search file contents | P3 | High |

### Command Implementations

```rust
// kernel/src/shell/commands.rs (additions)

/// Print working directory
pub fn pwd(_args: &[&str]) {
    match crate::fs::getcwd() {
        Ok(path) => println!("{}", path),
        Err(e) => println!("pwd: {}", e),
    }
}

/// List directory contents
pub fn ls(args: &[&str]) {
    let path = args.first().map(|s| *s).unwrap_or(".");
    
    match crate::fs::readdir(path) {
        Ok(entries) => {
            for entry in entries {
                let type_char = match entry.inode_type {
                    InodeType::Directory => 'd',
                    InodeType::File => '-',
                    InodeType::Symlink => 'l',
                };
                println!("{} {:>8}  {}", type_char, entry.size, entry.name);
            }
        }
        Err(e) => println!("ls: {}: {}", path, e),
    }
}

/// Change directory
pub fn cd(args: &[&str]) {
    let path = args.first().map(|s| *s).unwrap_or("/");
    
    if let Err(e) = crate::fs::chdir(path) {
        println!("cd: {}: {}", path, e);
    }
}

/// Create directory
pub fn mkdir(args: &[&str]) {
    if args.is_empty() {
        println!("mkdir: missing operand");
        return;
    }
    
    for path in args {
        if let Err(e) = crate::fs::mkdir(path) {
            println!("mkdir: {}: {}", path, e);
        }
    }
}

/// Create empty file or update timestamp
pub fn touch(args: &[&str]) {
    if args.is_empty() {
        println!("touch: missing operand");
        return;
    }
    
    for path in args {
        if let Err(e) = crate::fs::touch(path) {
            println!("touch: {}: {}", path, e);
        }
    }
}

/// Display file contents
pub fn cat(args: &[&str]) {
    if args.is_empty() {
        println!("cat: missing operand");
        return;
    }
    
    for path in args {
        match crate::fs::read_to_string(path) {
            Ok(contents) => print!("{}", contents),
            Err(e) => println!("cat: {}: {}", path, e),
        }
    }
}

/// Remove file
pub fn rm(args: &[&str]) {
    if args.is_empty() {
        println!("rm: missing operand");
        return;
    }
    
    for path in args {
        if let Err(e) = crate::fs::unlink(path) {
            println!("rm: {}: {}", path, e);
        }
    }
}

/// Remove directory
pub fn rmdir(args: &[&str]) {
    if args.is_empty() {
        println!("rmdir: missing operand");
        return;
    }
    
    for path in args {
        if let Err(e) = crate::fs::rmdir(path) {
            println!("rmdir: {}: {}", path, e);
        }
    }
}

/// Write text to file (simple version for shell)
pub fn write_file(args: &[&str]) {
    // Usage: write <filename> <content>
    if args.len() < 2 {
        println!("write: usage: write <filename> <content>");
        return;
    }
    
    let path = args[0];
    let content = args[1..].join(" ");
    
    if let Err(e) = crate::fs::write_string(path, &content) {
        println!("write: {}: {}", path, e);
    }
}
```

### Updated Help Command

```rust
pub fn help(_args: &[&str]) {
    println!("DebOS Shell Commands:");
    println!("=====================");
    println!();
    println!("System Commands:");
    println!("  help, ?        - Show this help message");
    println!("  info, sysinfo  - Display system information");
    println!("  mem, memory    - Show memory statistics");
    println!("  threads, ps    - List running threads");
    println!("  uptime         - Show system uptime");
    println!("  clear, cls     - Clear the screen");
    println!("  exit, quit     - Exit the shell");
    println!("  reboot         - Reboot the system");
    println!();
    println!("Filesystem Commands:");
    println!("  pwd            - Print working directory");
    println!("  ls [path]      - List directory contents");
    println!("  cd <path>      - Change directory");
    println!("  mkdir <path>   - Create directory");
    println!("  rmdir <path>   - Remove empty directory");
    println!("  touch <file>   - Create empty file");
    println!("  cat <file>     - Display file contents");
    println!("  rm <file>      - Remove file");
    println!("  write <f> <t>  - Write text to file");
    println!();
    println!("Other:");
    println!("  echo <text>    - Echo text to the console");
    println!();
}
```

---

## API Reference

### VFS IPC Protocol (Future - Userspace VFS)

```rust
/// VFS Request Messages
#[repr(u32)]
pub enum VfsRequest {
    // File Operations
    Open { path: [u8; 256], flags: u32 } = 1,
    Close { fd: u32 } = 2,
    Read { fd: u32, len: u32 } = 3,
    Write { fd: u32, data: [u8; 4096] } = 4,
    Seek { fd: u32, offset: i64, whence: u32 } = 5,
    
    // Directory Operations
    Mkdir { path: [u8; 256] } = 10,
    Rmdir { path: [u8; 256] } = 11,
    Readdir { path: [u8; 256] } = 12,
    
    // Path Operations
    Unlink { path: [u8; 256] } = 20,
    Rename { old: [u8; 256], new: [u8; 256] } = 21,
    Stat { path: [u8; 256] } = 22,
    
    // Working Directory
    Chdir { path: [u8; 256] } = 30,
    Getcwd = 31,
    
    // Mount Operations
    Mount { device: [u8; 64], mountpoint: [u8; 256], fstype: [u8; 32] } = 40,
    Unmount { path: [u8; 256] } = 41,
}

/// VFS Response Messages
pub enum VfsResponse {
    Ok,
    OkFd(u32),
    OkData(Vec<u8>),
    OkPath(String),
    OkStat(Stat),
    OkDirEntries(Vec<DirEntry>),
    Error(FsError),
}
```

### Error Types

```rust
#[derive(Debug, Clone, Copy)]
pub enum FsError {
    NotFound,           // File or directory not found
    AlreadyExists,      // File or directory already exists
    NotADirectory,      // Expected directory, got file
    IsADirectory,       // Expected file, got directory
    NotEmpty,           // Directory not empty (for rmdir)
    PermissionDenied,   // Access denied
    InvalidPath,        // Malformed path
    NoSpace,            // No space left on device
    ReadOnly,           // Read-only filesystem
    IoError,            // Generic I/O error
    InvalidFd,          // Invalid file descriptor
    TooManyOpenFiles,   // Too many open file handles
}
```

---

## Testing Strategy

### Unit Tests (In-Kernel)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mkdir_and_ls() {
        let mut fs = RamFs::new();
        fs.mkdir("/test").unwrap();
        let entries = fs.readdir("/").unwrap();
        assert!(entries.iter().any(|e| e.name == "test"));
    }
    
    #[test]
    fn test_file_read_write() {
        let mut fs = RamFs::new();
        fs.touch("/hello.txt").unwrap();
        fs.write_string("/hello.txt", "Hello, DebOS!").unwrap();
        let content = fs.read_to_string("/hello.txt").unwrap();
        assert_eq!(content, "Hello, DebOS!");
    }
    
    #[test]
    fn test_path_resolution() {
        assert_eq!(normalize("/a/b/../c"), "/a/c");
        assert_eq!(normalize("/a/./b"), "/a/b");
        assert_eq!(normalize("//a///b//"), "/a/b");
    }
}
```

### Integration Tests (Shell)

Run in QEMU and verify:

```
debos> mkdir /home
debos> mkdir /home/user
debos> cd /home/user
debos> pwd
/home/user
debos> touch hello.txt
debos> write hello.txt Hello World
debos> cat hello.txt
Hello World
debos> ls
-        11  hello.txt
debos> cd ..
debos> ls
d         0  user
debos> rm /home/user/hello.txt
debos> rmdir /home/user
debos> rmdir /home
```

---

## Implementation Checklist

### Week 1: Core RamFS
- [ ] Create `kernel/src/fs/mod.rs`
- [ ] Implement `Inode` and `FileHandle` structures
- [ ] Implement `RamFs::new()` with root directory
- [ ] Implement `create()`, `lookup()`, `readdir()`
- [ ] Add basic error handling

### Week 2: VFS & Shell Commands
- [ ] Implement `Vfs` layer with thread-local state
- [ ] Implement `open()`, `close()`, `read()`, `write()`
- [ ] Add `pwd`, `ls`, `cd`, `mkdir` commands
- [ ] Add `touch`, `cat`, `rm`, `rmdir` commands
- [ ] Update shell help

### Week 3: Path Resolution & Polish
- [ ] Implement full path normalization
- [ ] Handle relative paths with CWD
- [ ] Add `write` command for shell
- [ ] Error message improvements
- [ ] Edge case handling

### Week 4: VFS Server Migration
- [ ] Create VFS server process skeleton
- [ ] Define IPC protocol messages
- [ ] Implement server-side handlers
- [ ] Implement libdebos client stubs
- [ ] Test IPC communication

### Week 5-6: Block Devices & FAT32
- [ ] Implement block device abstraction
- [ ] VirtIO-Block driver
- [ ] FAT32 read support
- [ ] FAT32 write support
- [ ] Mount/unmount commands

---

## References

- POSIX Filesystem Specification
- Linux VFS documentation
- Redox OS filesystem implementation
- FAT32 Specification (Microsoft)
- ext4 Disk Layout (kernel.org)

---

*Document Version: 1.0.0*  
*Last Updated: November 2024*

