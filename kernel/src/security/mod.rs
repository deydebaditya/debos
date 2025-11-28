//! DebOS Security Subsystem
//!
//! Provides user management, authentication, authorization, and security policies.
//!
//! ## Design Philosophy
//! - **Simple to configure**: Admins can easily set up policies
//! - **Abstracted complexity**: Users don't need to understand internals
//! - **RBAC-based**: Role-based access control with policy guardrails
//! - **Defense in depth**: Multiple security layers
//!
//! ## Default Configuration
//! - Default user: `debos` (uid=1000, admin, no password)
//! - Admin group: `wheel` (gid=10)
//! - Any user can become superuser with proper authentication and RBAC policies

pub mod identity;
pub mod credentials;
pub mod auth;
pub mod policy;
pub mod database;
pub mod capability;
pub mod argon2;

use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;

pub use identity::{UserId, GroupId, User, Group};
pub use credentials::ProcessCredentials;
pub use auth::{authenticate, AuthResult};
pub use policy::{SecurityPolicy, ResourceLimits};
pub use capability::{Capability, CapabilitySet};

/// Initialize the security subsystem
pub fn init() {
    crate::println!("[..] Initializing security subsystem...");
    
    // Initialize user database with defaults
    database::init();
    
    // Load security policies
    policy::init();
    
    crate::println!("[OK] Security subsystem initialized");
    crate::println!("     Default user: debos (admin, no password)");
}

/// Get current user ID from running process
pub fn current_uid() -> UserId {
    if let Some(creds) = crate::scheduler::current_credentials() {
        creds.uid
    } else {
        UserId::ROOT // Kernel context
    }
}

/// Get current effective user ID
pub fn current_euid() -> UserId {
    if let Some(creds) = crate::scheduler::current_credentials() {
        creds.euid
    } else {
        UserId::ROOT
    }
}

/// Check if current process is running as root
pub fn is_root() -> bool {
    current_euid() == UserId::ROOT
}

/// Check if current process has a specific capability
pub fn has_capability(cap: Capability) -> bool {
    if let Some(creds) = crate::scheduler::current_credentials() {
        creds.capabilities.contains(cap)
    } else {
        true // Kernel has all capabilities
    }
}

