//! Security Policies
//!
//! Defines security policies, resource limits, and access controls.
//!
//! ## RBAC (Role-Based Access Control)
//! - **Admin**: Full system access (wheel group)
//! - **User**: Standard user access
//! - **Service**: Limited service account access
//! - **Guest**: Minimal access

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

use super::identity::{UserId, GroupId};

/// Role definition for RBAC
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// System administrator (wheel group)
    Admin,
    /// Regular user
    User,
    /// Service account (daemons)
    Service,
    /// Guest/restricted user
    Guest,
}

impl Role {
    /// Get resource limits for this role
    pub fn limits(&self) -> ResourceLimits {
        match self {
            Role::Admin => ResourceLimits::admin(),
            Role::User => ResourceLimits::user(),
            Role::Service => ResourceLimits::service(),
            Role::Guest => ResourceLimits::guest(),
        }
    }
    
    /// Can this role become root (via sudo)?
    pub fn can_sudo(&self) -> bool {
        matches!(self, Role::Admin)
    }
    
    /// Can this role manage users?
    pub fn can_manage_users(&self) -> bool {
        matches!(self, Role::Admin)
    }
    
    /// Can this role manage services?
    pub fn can_manage_services(&self) -> bool {
        matches!(self, Role::Admin | Role::Service)
    }
}

/// Resource limits for a user/process
#[derive(Clone, Debug)]
pub struct ResourceLimits {
    /// Maximum number of processes
    pub max_processes: u32,
    
    /// Maximum open files
    pub max_open_files: u32,
    
    /// Maximum file size (bytes)
    pub max_file_size: u64,
    
    /// Maximum memory (bytes)
    pub max_memory: u64,
    
    /// Maximum CPU time (seconds, 0 = unlimited)
    pub max_cpu_time: u64,
    
    /// Maximum threads per process
    pub max_threads: u32,
    
    /// Nice priority range (min, max)
    pub nice_range: (i8, i8),
    
    /// Can use real-time scheduling
    pub allow_realtime: bool,
    
    /// Can lock memory (mlock)
    pub allow_mlock: bool,
}

impl ResourceLimits {
    /// Unlimited resources (for kernel/root)
    pub fn unlimited() -> Self {
        ResourceLimits {
            max_processes: u32::MAX,
            max_open_files: u32::MAX,
            max_file_size: u64::MAX,
            max_memory: u64::MAX,
            max_cpu_time: 0,  // 0 = unlimited
            max_threads: u32::MAX,
            nice_range: (-20, 19),
            allow_realtime: true,
            allow_mlock: true,
        }
    }
    
    /// Admin limits (generous but not unlimited)
    pub fn admin() -> Self {
        ResourceLimits {
            max_processes: 8192,
            max_open_files: 65536,
            max_file_size: 100 * 1024 * 1024 * 1024,  // 100 GB
            max_memory: 32 * 1024 * 1024 * 1024,      // 32 GB
            max_cpu_time: 0,
            max_threads: 4096,
            nice_range: (-20, 19),
            allow_realtime: true,
            allow_mlock: true,
        }
    }
    
    /// Standard user limits
    pub fn user() -> Self {
        ResourceLimits {
            max_processes: 1024,
            max_open_files: 4096,
            max_file_size: 10 * 1024 * 1024 * 1024,  // 10 GB
            max_memory: 8 * 1024 * 1024 * 1024,      // 8 GB
            max_cpu_time: 0,
            max_threads: 256,
            nice_range: (0, 19),  // Can only lower priority
            allow_realtime: false,
            allow_mlock: false,
        }
    }
    
    /// Service account limits
    pub fn service() -> Self {
        ResourceLimits {
            max_processes: 256,
            max_open_files: 8192,
            max_file_size: 50 * 1024 * 1024 * 1024,  // 50 GB
            max_memory: 16 * 1024 * 1024 * 1024,     // 16 GB
            max_cpu_time: 0,
            max_threads: 512,
            nice_range: (-5, 19),
            allow_realtime: true,
            allow_mlock: true,
        }
    }
    
    /// Guest (restricted) limits
    pub fn guest() -> Self {
        ResourceLimits {
            max_processes: 64,
            max_open_files: 256,
            max_file_size: 1 * 1024 * 1024 * 1024,   // 1 GB
            max_memory: 1 * 1024 * 1024 * 1024,      // 1 GB
            max_cpu_time: 3600,  // 1 hour
            max_threads: 32,
            nice_range: (10, 19),  // Low priority only
            allow_realtime: false,
            allow_mlock: false,
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::user()
    }
}

/// Security policy configuration
#[derive(Clone)]
pub struct SecurityPolicy {
    /// Require password for sudo (even for admins)
    pub sudo_requires_password: bool,
    
    /// Allow empty passwords
    pub allow_empty_password: bool,
    
    /// Minimum password length (0 = no minimum)
    pub min_password_length: u8,
    
    /// Password expiry days (0 = never)
    pub password_expiry_days: u32,
    
    /// Failed login lockout threshold
    pub lockout_threshold: u32,
    
    /// Lockout duration (seconds)
    pub lockout_duration: u64,
    
    /// Allow root login
    pub allow_root_login: bool,
    
    /// Secure terminals for root (empty = all)
    pub root_terminals: Vec<String>,
    
    /// Enable audit logging
    pub audit_enabled: bool,
    
    /// Log successful logins
    pub log_successful_logins: bool,
    
    /// Log failed logins
    pub log_failed_logins: bool,
    
    /// Log sudo usage
    pub log_sudo: bool,
}

impl SecurityPolicy {
    /// Default security policy (balanced security/usability)
    pub fn default_policy() -> Self {
        SecurityPolicy {
            sudo_requires_password: true,
            allow_empty_password: true,  // For debos user
            min_password_length: 0,
            password_expiry_days: 0,
            lockout_threshold: 5,
            lockout_duration: 300,
            allow_root_login: false,  // Root login disabled
            root_terminals: Vec::new(),
            audit_enabled: true,
            log_successful_logins: true,
            log_failed_logins: true,
            log_sudo: true,
        }
    }
    
    /// Strict security policy
    pub fn strict() -> Self {
        SecurityPolicy {
            sudo_requires_password: true,
            allow_empty_password: false,
            min_password_length: 12,
            password_expiry_days: 90,
            lockout_threshold: 3,
            lockout_duration: 900,
            allow_root_login: false,
            root_terminals: Vec::new(),
            audit_enabled: true,
            log_successful_logins: true,
            log_failed_logins: true,
            log_sudo: true,
        }
    }
    
    /// Relaxed security policy (for development/testing)
    pub fn relaxed() -> Self {
        SecurityPolicy {
            sudo_requires_password: false,
            allow_empty_password: true,
            min_password_length: 0,
            password_expiry_days: 0,
            lockout_threshold: 100,
            lockout_duration: 60,
            allow_root_login: true,
            root_terminals: Vec::new(),
            audit_enabled: false,
            log_successful_logins: false,
            log_failed_logins: true,
            log_sudo: false,
        }
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::default_policy()
    }
}

lazy_static! {
    /// Global security policy
    static ref SECURITY_POLICY: Mutex<SecurityPolicy> = Mutex::new(SecurityPolicy::default());
    
    /// Per-user resource limits (username -> limits)
    static ref USER_LIMITS: Mutex<BTreeMap<String, ResourceLimits>> = Mutex::new(BTreeMap::new());
    
    /// Per-group resource limits (group name -> limits)
    static ref GROUP_LIMITS: Mutex<BTreeMap<String, ResourceLimits>> = Mutex::new(BTreeMap::new());
}

/// Initialize security policies
pub fn init() {
    // Set up default group limits
    let mut group_limits = GROUP_LIMITS.lock();
    group_limits.insert(String::from("wheel"), ResourceLimits::admin());
    group_limits.insert(String::from("users"), ResourceLimits::user());
}

/// Get the current security policy
pub fn get_policy() -> SecurityPolicy {
    SECURITY_POLICY.lock().clone()
}

/// Update the security policy (requires admin)
pub fn set_policy(policy: SecurityPolicy) -> Result<(), &'static str> {
    if !super::is_root() && !super::has_capability(super::capability::Capability::DebosPolicy) {
        return Err("Permission denied: requires admin");
    }
    
    *SECURITY_POLICY.lock() = policy;
    Ok(())
}

/// Get resource limits for a user
pub fn get_limits_for_user(username: &str) -> ResourceLimits {
    // Check user-specific limits first
    if let Some(limits) = USER_LIMITS.lock().get(username) {
        return limits.clone();
    }
    
    // Check if user is admin
    if let Some(user) = super::database::get_user_by_name(username) {
        if user.is_admin {
            return ResourceLimits::admin();
        }
    }
    
    // Check group limits
    let groups = super::database::get_user_groups(username);
    for group in groups {
        if let Some(limits) = GROUP_LIMITS.lock().get(&group.name) {
            return limits.clone();
        }
    }
    
    // Default to standard user limits
    ResourceLimits::user()
}

/// Set resource limits for a user
pub fn set_limits_for_user(username: &str, limits: ResourceLimits) -> Result<(), &'static str> {
    if !super::is_root() {
        return Err("Permission denied: requires root");
    }
    
    USER_LIMITS.lock().insert(String::from(username), limits);
    Ok(())
}

/// Get role for a user
pub fn get_role(username: &str) -> Role {
    if let Some(user) = super::database::get_user_by_name(username) {
        if user.is_admin {
            Role::Admin
        } else if user.uid.is_system() {
            Role::Service
        } else {
            Role::User
        }
    } else {
        Role::Guest
    }
}

/// Check if an action is allowed by policy
pub fn check_policy_permission(action: PolicyAction, username: &str) -> bool {
    let policy = SECURITY_POLICY.lock();
    let role = get_role(username);
    
    match action {
        PolicyAction::Login => {
            // Check if user exists and account is active
            if let Some(user) = super::database::get_user_by_name(username) {
                user.status == super::identity::AccountStatus::Active
            } else {
                false
            }
        }
        PolicyAction::RootLogin => {
            policy.allow_root_login && username == "root"
        }
        PolicyAction::Sudo => {
            role.can_sudo()
        }
        PolicyAction::ManageUsers => {
            role.can_manage_users()
        }
        PolicyAction::ManageServices => {
            role.can_manage_services()
        }
        PolicyAction::ChangePolicy => {
            matches!(role, Role::Admin)
        }
    }
}

/// Policy actions that can be checked
#[derive(Clone, Copy, Debug)]
pub enum PolicyAction {
    Login,
    RootLogin,
    Sudo,
    ManageUsers,
    ManageServices,
    ChangePolicy,
}

/// Audit log entry
#[derive(Clone, Debug)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub event_type: AuditEventType,
    pub username: String,
    pub details: String,
    pub success: bool,
}

/// Audit event types
#[derive(Clone, Copy, Debug)]
pub enum AuditEventType {
    Login,
    Logout,
    LoginFailed,
    Sudo,
    SudoFailed,
    PasswordChange,
    UserCreated,
    UserDeleted,
    GroupChange,
    PolicyChange,
    PermissionDenied,
}

lazy_static! {
    /// Audit log (in-memory for now)
    static ref AUDIT_LOG: Mutex<Vec<AuditEntry>> = Mutex::new(Vec::new());
}

/// Log an audit event
pub fn audit_log(event_type: AuditEventType, username: &str, details: &str, success: bool) {
    let policy = SECURITY_POLICY.lock();
    
    // Check if we should log this event
    let should_log = match event_type {
        AuditEventType::Login | AuditEventType::Logout => policy.log_successful_logins,
        AuditEventType::LoginFailed => policy.log_failed_logins,
        AuditEventType::Sudo | AuditEventType::SudoFailed => policy.log_sudo,
        _ => policy.audit_enabled,
    };
    
    if !should_log {
        return;
    }
    
    drop(policy);
    
    let entry = AuditEntry {
        timestamp: 0,  // TODO: real timestamp
        event_type,
        username: String::from(username),
        details: String::from(details),
        success,
    };
    
    AUDIT_LOG.lock().push(entry);
}

/// Get recent audit entries
pub fn get_audit_log(count: usize) -> Vec<AuditEntry> {
    let log = AUDIT_LOG.lock();
    let start = if log.len() > count { log.len() - count } else { 0 };
    log[start..].to_vec()
}

