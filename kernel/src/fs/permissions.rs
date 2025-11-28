//! File Permission Checking
//!
//! Implements POSIX-style permission checking for filesystem operations.

use super::{Stat, InodeType, FsError, FsResult};

/// Access modes for permission checks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// Check for read permission
    Read,
    /// Check for write permission
    Write,
    /// Check for execute permission
    Execute,
    /// Check if file exists (no permission check)
    Exists,
}

/// Permission bits
pub mod perms {
    /// Owner can read
    pub const S_IRUSR: u16 = 0o400;
    /// Owner can write
    pub const S_IWUSR: u16 = 0o200;
    /// Owner can execute
    pub const S_IXUSR: u16 = 0o100;
    
    /// Group can read
    pub const S_IRGRP: u16 = 0o040;
    /// Group can write
    pub const S_IWGRP: u16 = 0o020;
    /// Group can execute
    pub const S_IXGRP: u16 = 0o010;
    
    /// Others can read
    pub const S_IROTH: u16 = 0o004;
    /// Others can write
    pub const S_IWOTH: u16 = 0o002;
    /// Others can execute
    pub const S_IXOTH: u16 = 0o001;
    
    /// Setuid bit
    pub const S_ISUID: u16 = 0o4000;
    /// Setgid bit
    pub const S_ISGID: u16 = 0o2000;
    /// Sticky bit
    pub const S_ISVTX: u16 = 0o1000;
    
    /// All permissions (rwxrwxrwx)
    pub const S_IRWXU: u16 = S_IRUSR | S_IWUSR | S_IXUSR;
    pub const S_IRWXG: u16 = S_IRGRP | S_IWGRP | S_IXGRP;
    pub const S_IRWXO: u16 = S_IROTH | S_IWOTH | S_IXOTH;
}

/// Check if the current user has the specified access to a file
/// 
/// Returns Ok(()) if access is allowed, Err(FsError::PermissionDenied) otherwise.
pub fn check_access(stat: &Stat, mode: AccessMode) -> FsResult<()> {
    // Get current credentials
    let (uid, gid, groups, has_dac_override) = get_current_credentials();
    
    // Existence check always succeeds if we can see the file
    if mode == AccessMode::Exists {
        return Ok(());
    }
    
    // Root (uid 0) with CAP_DAC_OVERRIDE bypasses permission checks
    // For read/write. Execute still requires at least one execute bit.
    if uid == 0 && has_dac_override {
        if mode == AccessMode::Execute {
            // Root can execute if any execute bit is set
            if stat.permissions & (perms::S_IXUSR | perms::S_IXGRP | perms::S_IXOTH) != 0 {
                return Ok(());
            }
            // Or for directories, always allow
            if stat.inode_type == InodeType::Directory {
                return Ok(());
            }
        } else {
            return Ok(());
        }
    }
    
    // Determine which permission bits to check
    let (read_bit, write_bit, exec_bit) = if uid == stat.uid {
        // Owner permissions
        (perms::S_IRUSR, perms::S_IWUSR, perms::S_IXUSR)
    } else if gid == stat.gid || groups.contains(&stat.gid) {
        // Group permissions
        (perms::S_IRGRP, perms::S_IWGRP, perms::S_IXGRP)
    } else {
        // Other permissions
        (perms::S_IROTH, perms::S_IWOTH, perms::S_IXOTH)
    };
    
    // Check the specific permission
    let required_bit = match mode {
        AccessMode::Read => read_bit,
        AccessMode::Write => write_bit,
        AccessMode::Execute => exec_bit,
        AccessMode::Exists => return Ok(()),
    };
    
    if stat.permissions & required_bit != 0 {
        Ok(())
    } else {
        Err(FsError::PermissionDenied)
    }
}

/// Check read access
pub fn check_read(stat: &Stat) -> FsResult<()> {
    check_access(stat, AccessMode::Read)
}

/// Check write access
pub fn check_write(stat: &Stat) -> FsResult<()> {
    check_access(stat, AccessMode::Write)
}

/// Check execute access
pub fn check_execute(stat: &Stat) -> FsResult<()> {
    check_access(stat, AccessMode::Execute)
}

/// Get current user credentials for permission checking
fn get_current_credentials() -> (u32, u32, alloc::vec::Vec<u32>, bool) {
    if let Some(creds) = crate::scheduler::current_credentials() {
        let uid = creds.euid.as_raw();
        let gid = creds.egid.as_raw();
        let groups = creds.groups.iter().map(|g| g.as_raw()).collect();
        let has_dac_override = creds.capabilities.contains(
            crate::security::capability::Capability::DacOverride
        );
        (uid, gid, groups, has_dac_override)
    } else {
        // Kernel context - full access
        (0, 0, alloc::vec::Vec::new(), true)
    }
}

/// Format permissions as a string (e.g., "rwxr-xr-x")
pub fn format_permissions(mode: u16) -> alloc::string::String {
    let mut s = alloc::string::String::with_capacity(9);
    
    // Owner
    s.push(if mode & perms::S_IRUSR != 0 { 'r' } else { '-' });
    s.push(if mode & perms::S_IWUSR != 0 { 'w' } else { '-' });
    s.push(if mode & perms::S_IXUSR != 0 {
        if mode & perms::S_ISUID != 0 { 's' } else { 'x' }
    } else {
        if mode & perms::S_ISUID != 0 { 'S' } else { '-' }
    });
    
    // Group
    s.push(if mode & perms::S_IRGRP != 0 { 'r' } else { '-' });
    s.push(if mode & perms::S_IWGRP != 0 { 'w' } else { '-' });
    s.push(if mode & perms::S_IXGRP != 0 {
        if mode & perms::S_ISGID != 0 { 's' } else { 'x' }
    } else {
        if mode & perms::S_ISGID != 0 { 'S' } else { '-' }
    });
    
    // Others
    s.push(if mode & perms::S_IROTH != 0 { 'r' } else { '-' });
    s.push(if mode & perms::S_IWOTH != 0 { 'w' } else { '-' });
    s.push(if mode & perms::S_IXOTH != 0 {
        if mode & perms::S_ISVTX != 0 { 't' } else { 'x' }
    } else {
        if mode & perms::S_ISVTX != 0 { 'T' } else { '-' }
    });
    
    s
}

/// Parse a numeric mode string (e.g., "755") to permissions
pub fn parse_mode(s: &str) -> Option<u16> {
    u16::from_str_radix(s, 8).ok()
}

/// Apply umask to a mode
pub fn apply_umask(mode: u16, umask: u16) -> u16 {
    mode & !umask
}

/// Get default file permissions (with umask)
pub fn default_file_mode() -> u16 {
    apply_umask(0o666, get_umask())
}

/// Get default directory permissions (with umask)
pub fn default_dir_mode() -> u16 {
    apply_umask(0o777, get_umask())
}

/// Get current umask
fn get_umask() -> u16 {
    // Default umask of 022
    // TODO: Store per-process umask
    0o022
}

/// Check if user can change ownership (requires CAP_CHOWN)
pub fn can_chown() -> bool {
    if let Some(creds) = crate::scheduler::current_credentials() {
        creds.euid.as_raw() == 0 || 
        creds.capabilities.contains(crate::security::capability::Capability::Chown)
    } else {
        true // Kernel context
    }
}

/// Check if user can change mode (must be owner or root)
pub fn can_chmod(stat: &Stat) -> bool {
    if let Some(creds) = crate::scheduler::current_credentials() {
        let uid = creds.euid.as_raw();
        uid == 0 || uid == stat.uid
    } else {
        true // Kernel context
    }
}

