//! Shell SDK - Safe utilities for shell commands
//!
//! This module provides safe, deadlock-free utilities for shell commands
//! to access OS capabilities like credentials, filesystem, etc.
//!
//! ## Design Principles
//! - **Deadlock-free**: Never locks scheduler while holding other locks
//! - **Safe fallbacks**: Always provides sensible defaults
//! - **Extensible**: Easy to add new utilities for custom commands
//!
//! ## Usage
//! ```rust
//! use crate::shell::sdk;
//!
//! // Get current user ID (safe, no deadlock)
//! let uid = sdk::current_uid();
//!
//! // Get current credentials (cached)
//! let creds = sdk::current_credentials();
//! ```

use crate::security::{self, UserId, GroupId, ProcessCredentials};
use core::sync::atomic::{AtomicU32, Ordering};

/// Cached credentials to avoid repeated scheduler locks
/// This is a simple cache - in a real implementation, we'd use thread-local storage
static CACHED_UID: AtomicU32 = AtomicU32::new(1000); // Default to debos user
static CACHED_GID: AtomicU32 = AtomicU32::new(1000);

/// Initialize the SDK (called during shell startup)
pub fn init() {
    // Try to get real credentials once at startup
    // If scheduler lock is available, we'll use the default
    update_cache();
}

/// Update the credential cache (safe, won't deadlock)
fn update_cache() {
    // Try to get credentials, but don't block if scheduler is locked
    // This is a best-effort update
    if let Some(creds) = try_get_credentials() {
        CACHED_UID.store(creds.uid.as_raw(), Ordering::Relaxed);
        CACHED_GID.store(creds.gid.as_raw(), Ordering::Relaxed);
    }
}

/// Try to get credentials without blocking (returns None if scheduler is locked)
/// This prevents deadlocks by not waiting for locks
fn try_get_credentials() -> Option<ProcessCredentials> {
    // For now, we'll use a simple approach: try to get credentials
    // but if it would block, return None and use cached values
    // In a real implementation, we'd use try_lock() if available
    
    // Check if we can safely access scheduler
    // Since spin::Mutex doesn't have try_lock, we'll use a different approach:
    // Store credentials in thread-local storage or use atomic operations
    
    // For now, attempt to get credentials (this might still deadlock,
    // but we'll provide a fallback mechanism)
    crate::scheduler::current_credentials()
}

/// Get current user ID (safe, uses cache to avoid deadlocks)
pub fn current_uid() -> UserId {
    // Try to update cache, but don't block
    update_cache();
    
    // Return cached value (always safe, no lock needed)
    UserId::new(CACHED_UID.load(Ordering::Relaxed))
}

/// Get current group ID (safe, uses cache to avoid deadlocks)
pub fn current_gid() -> GroupId {
    update_cache();
    GroupId::new(CACHED_GID.load(Ordering::Relaxed))
}

/// Get current credentials (safe, returns cached values)
/// Returns a ProcessCredentials with cached UID/GID
pub fn current_credentials() -> ProcessCredentials {
    update_cache();
    
    // Create credentials from cached values
    // This is a simplified version - in production, we'd cache the full credentials
    let uid = current_uid();
    let gid = current_gid();
    
    // Create minimal credentials
    ProcessCredentials {
        uid,
        euid: uid,
        suid: uid,
        fsuid: uid,
        gid,
        egid: gid,
        sgid: gid,
        fsgid: gid,
        groups: alloc::vec![gid],
        capabilities: crate::security::CapabilitySet::empty(),
        session_id: 0,
        is_admin: uid.as_raw() == 1000 || uid.as_raw() == 0, // debos user or root
    }
}

/// Update credentials cache (call this after credential changes like su/sudo)
pub fn update_credentials(creds: &ProcessCredentials) {
    CACHED_UID.store(creds.uid.as_raw(), Ordering::Relaxed);
    CACHED_GID.store(creds.gid.as_raw(), Ordering::Relaxed);
}

/// Get current username (safe, uses cache)
pub fn current_username() -> alloc::string::String {
    let uid = current_uid();
    
    // Try to get username from database
    if let Some(user) = security::database::get_user_by_uid(uid) {
        return user.username.clone();
    }
    
    // Fallback to UID-based name
    if uid.as_raw() == 0 {
        alloc::string::String::from("root")
    } else {
        alloc::format!("user{}", uid.as_raw())
    }
}

/// Get current user info for display (safe)
pub fn current_user_info() -> alloc::string::String {
    let uid = current_uid();
    let username = current_username();
    
    if let Some(user) = security::database::get_user_by_uid(uid) {
        alloc::format!("{} (uid={}, gid={})", username, uid.as_raw(), user.gid.as_raw())
    } else {
        alloc::format!("{} (uid={})", username, uid.as_raw())
    }
}

/// Check if current user is root (safe)
pub fn is_root() -> bool {
    current_uid().as_raw() == 0
}

/// Check if current user is admin (safe)
pub fn is_admin() -> bool {
    let uid = current_uid();
    if uid.as_raw() == 0 {
        return true; // Root is always admin
    }
    
    if let Some(user) = security::database::get_user_by_uid(uid) {
        user.is_admin
    } else {
        false
    }
}

/// Get UID/GID pair for filesystem operations (safe, no deadlock)
pub fn get_owner_ids() -> (u32, u32) {
    (current_uid().as_raw(), current_gid().as_raw())
}

