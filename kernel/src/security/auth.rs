//! Authentication System
//!
//! Handles user authentication, password management, and session control.
//!
//! ## Password Security
//! - Passwords are hashed using Argon2id (memory-hard algorithm)
//! - No plaintext passwords are ever stored
//! - Constant-time comparison to prevent timing attacks
//! - 128-bit random salts per password
//!
//! ## Default Configuration
//! - `debos` user has no password (empty password allowed)
//! - `root` has no password by default (disabled for login)
//! - Custom users must have passwords

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;

use super::identity::{UserId, User, AccountStatus};
use super::credentials::ProcessCredentials;
use super::database;

/// Maximum failed login attempts before lockout
pub const MAX_FAILED_ATTEMPTS: u32 = 5;

/// Lockout duration in seconds
pub const LOCKOUT_DURATION_SECS: u64 = 300; // 5 minutes

/// Authentication result
#[derive(Debug, Clone)]
pub enum AuthResult {
    /// Authentication successful
    Success(User),
    /// Invalid username
    UserNotFound,
    /// Invalid password
    InvalidPassword,
    /// Account is locked
    AccountLocked,
    /// Account is disabled
    AccountDisabled,
    /// Account has expired
    AccountExpired,
    /// No password set (for password-less accounts like debos)
    NoPasswordRequired(User),
}

/// Password hash entry
#[derive(Clone)]
pub struct PasswordEntry {
    /// Username
    pub username: String,
    
    /// Password hash (SHA-256 of salt + password)
    /// Empty means no password required
    pub hash: Vec<u8>,
    
    /// Salt for password hashing
    pub salt: [u8; 16],
    
    /// Is password set?
    pub has_password: bool,
    
    /// Failed login attempts
    pub failed_attempts: u32,
    
    /// Last failed login timestamp
    pub last_failed: u64,
    
    /// Account locked until (timestamp)
    pub locked_until: u64,
}

impl PasswordEntry {
    /// Create a new password entry with no password
    pub fn no_password(username: &str) -> Self {
        PasswordEntry {
            username: String::from(username),
            hash: Vec::new(),
            salt: [0u8; 16],
            has_password: false,
            failed_attempts: 0,
            last_failed: 0,
            locked_until: 0,
        }
    }
    
    /// Create a new password entry with a password
    pub fn with_password(username: &str, password: &str) -> Self {
        let salt = generate_salt();
        let hash = hash_password(password, &salt);
        
        PasswordEntry {
            username: String::from(username),
            hash,
            salt,
            has_password: true,
            failed_attempts: 0,
            last_failed: 0,
            locked_until: 0,
        }
    }
    
    /// Set a new password
    pub fn set_password(&mut self, password: &str) {
        self.salt = generate_salt();
        self.hash = hash_password(password, &self.salt);
        self.has_password = true;
        self.failed_attempts = 0;
    }
    
    /// Remove password (make password-less)
    pub fn remove_password(&mut self) {
        self.hash.clear();
        self.salt = [0u8; 16];
        self.has_password = false;
    }
    
    /// Verify password using Argon2id
    pub fn verify(&self, password: &str) -> bool {
        if !self.has_password {
            // No password required, any input is valid (including empty)
            return true;
        }
        
        verify_password(password, &self.salt, &self.hash)
    }
    
    /// Check if account is locked
    pub fn is_locked(&self, current_time: u64) -> bool {
        self.locked_until > current_time
    }
    
    /// Record failed login attempt
    pub fn record_failure(&mut self, current_time: u64) {
        self.failed_attempts += 1;
        self.last_failed = current_time;
        
        if self.failed_attempts >= MAX_FAILED_ATTEMPTS {
            self.locked_until = current_time + LOCKOUT_DURATION_SECS;
        }
    }
    
    /// Reset failed attempts (after successful login)
    pub fn reset_failures(&mut self) {
        self.failed_attempts = 0;
        self.locked_until = 0;
    }
}

/// Generate a random salt (pseudo-random for now)
fn generate_salt() -> [u8; 16] {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    
    // Simple pseudo-random based on counter and timestamp
    // In production, use hardware RNG
    let mut salt = [0u8; 16];
    let time_bytes = get_current_time().to_le_bytes();
    let count_bytes = count.to_le_bytes();
    
    for i in 0..8 {
        salt[i] = time_bytes[i];
        salt[i + 8] = count_bytes[i % 4].wrapping_add(time_bytes[i]);
    }
    
    // Mix the bytes
    for i in 0..16 {
        salt[i] = salt[i].wrapping_mul(31).wrapping_add(salt[(i + 7) % 16]);
    }
    
    salt
}

/// Hash password with salt using Argon2id
fn hash_password(password: &str, salt: &[u8; 16]) -> Vec<u8> {
    use super::argon2::{argon2id_hash, Argon2Params};
    
    // Use interactive parameters for login (faster but still secure)
    let params = Argon2Params::interactive();
    argon2id_hash(password.as_bytes(), salt, &params)
}

/// Verify password against stored hash using Argon2id
fn verify_password(password: &str, salt: &[u8; 16], expected: &[u8]) -> bool {
    use super::argon2::{argon2id_verify, Argon2Params};
    
    let params = Argon2Params::interactive();
    argon2id_verify(password.as_bytes(), salt, expected, &params)
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    
    diff == 0
}

/// Get current time (placeholder - returns tick count)
fn get_current_time() -> u64 {
    // In production, use real-time clock
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst) as u64
}

/// Authenticate a user
pub fn authenticate(username: &str, password: &str) -> AuthResult {
    // Look up user
    let user = match database::get_user_by_name(username) {
        Some(u) => u,
        None => return AuthResult::UserNotFound,
    };
    
    // Check account status
    match user.status {
        AccountStatus::Disabled => return AuthResult::AccountDisabled,
        AccountStatus::Expired => return AuthResult::AccountExpired,
        AccountStatus::Locked => return AuthResult::AccountLocked,
        AccountStatus::Active => {}
    }
    
    // Get password entry
    let mut password_db = database::PASSWORD_DB.lock();
    
    if let Some(entry) = password_db.get_mut(username) {
        let current_time = get_current_time();
        
        // Check if locked
        if entry.is_locked(current_time) {
            return AuthResult::AccountLocked;
        }
        
        // Check if password required
        if !entry.has_password {
            entry.reset_failures();
            return AuthResult::NoPasswordRequired(user);
        }
        
        // Verify password
        if entry.verify(password) {
            entry.reset_failures();
            AuthResult::Success(user)
        } else {
            entry.record_failure(current_time);
            AuthResult::InvalidPassword
        }
    } else {
        // No password entry - check if it's the debos user
        if username == "debos" {
            return AuthResult::NoPasswordRequired(user);
        }
        AuthResult::InvalidPassword
    }
}

/// Check if user can authenticate without password
pub fn is_passwordless(username: &str) -> bool {
    let password_db = database::PASSWORD_DB.lock();
    
    if let Some(entry) = password_db.get(username) {
        !entry.has_password
    } else {
        // debos user is passwordless by default
        username == "debos"
    }
}

/// Set password for user (requires appropriate permissions)
pub fn set_password(username: &str, new_password: &str) -> Result<(), &'static str> {
    let mut password_db = database::PASSWORD_DB.lock();
    
    if let Some(entry) = password_db.get_mut(username) {
        if new_password.is_empty() {
            entry.remove_password();
        } else {
            entry.set_password(new_password);
        }
        Ok(())
    } else {
        // Create new entry
        let entry = if new_password.is_empty() {
            PasswordEntry::no_password(username)
        } else {
            PasswordEntry::with_password(username, new_password)
        };
        password_db.insert(String::from(username), entry);
        Ok(())
    }
}

/// Unlock a locked account (admin function)
pub fn unlock_account(username: &str) -> Result<(), &'static str> {
    let mut password_db = database::PASSWORD_DB.lock();
    
    if let Some(entry) = password_db.get_mut(username) {
        entry.reset_failures();
        Ok(())
    } else {
        Err("User not found")
    }
}

/// Session tracking
static NEXT_SESSION_ID: AtomicU32 = AtomicU32::new(1);

/// Active session storage (persists across shell restarts)
use alloc::collections::BTreeMap;
use lazy_static::lazy_static;

lazy_static! {
    /// Active sessions by session ID
    static ref ACTIVE_SESSIONS: Mutex<BTreeMap<u32, SessionInfo>> = Mutex::new(BTreeMap::new());
    
    /// Current session for the console
    static ref CONSOLE_SESSION: Mutex<Option<u32>> = Mutex::new(None);
}

/// Session information
#[derive(Clone)]
pub struct SessionInfo {
    /// Session ID
    pub id: u32,
    /// Username
    pub username: String,
    /// User ID
    pub uid: super::identity::UserId,
    /// Group ID
    pub gid: super::identity::GroupId,
    /// Is admin
    pub is_admin: bool,
    /// Login time (ticks)
    pub login_time: u64,
    /// Last activity time
    pub last_activity: u64,
}

/// Create a new session for authenticated user
pub fn create_session(user: &User) -> ProcessCredentials {
    let session_id = NEXT_SESSION_ID.fetch_add(1, Ordering::SeqCst);
    
    let mut creds = ProcessCredentials::for_user(
        user.uid,
        user.gid,
        user.groups.clone(),
        user.is_admin,
    );
    creds.session_id = session_id;
    
    // Store session info
    let session_info = SessionInfo {
        id: session_id,
        username: user.username.clone(),
        uid: user.uid,
        gid: user.gid,
        is_admin: user.is_admin,
        login_time: crate::scheduler::ticks(),
        last_activity: crate::scheduler::ticks(),
    };
    
    ACTIVE_SESSIONS.lock().insert(session_id, session_info);
    
    creds
}

/// Set the console session (persists the current user)
pub fn set_console_session(session_id: u32) {
    *CONSOLE_SESSION.lock() = Some(session_id);
}

/// Get the console session
pub fn get_console_session() -> Option<u32> {
    *CONSOLE_SESSION.lock()
}

/// Get session info by ID
pub fn get_session(session_id: u32) -> Option<SessionInfo> {
    ACTIVE_SESSIONS.lock().get(&session_id).cloned()
}

/// End a session
pub fn end_session(session_id: u32) {
    ACTIVE_SESSIONS.lock().remove(&session_id);
    
    // Clear console session if it matches
    let mut console = CONSOLE_SESSION.lock();
    if *console == Some(session_id) {
        *console = None;
    }
}

/// Update session activity time
pub fn touch_session(session_id: u32) {
    if let Some(session) = ACTIVE_SESSIONS.lock().get_mut(&session_id) {
        session.last_activity = crate::scheduler::ticks();
    }
}

/// Get all active sessions
pub fn list_sessions() -> Vec<SessionInfo> {
    ACTIVE_SESSIONS.lock().values().cloned().collect()
}

/// Restore session from console (for shell restarts)
pub fn restore_console_session() -> Option<ProcessCredentials> {
    let session_id = get_console_session()?;
    let session = get_session(session_id)?;
    
    // Look up user
    let user = database::get_user_by_name(&session.username)?;
    
    // Update activity
    touch_session(session_id);
    
    // Create credentials
    let mut creds = ProcessCredentials::for_user(
        user.uid,
        user.gid,
        user.groups.clone(),
        user.is_admin,
    );
    creds.session_id = session_id;
    
    Some(creds)
}

/// Validate sudo access (user must be admin and authenticate)
pub fn validate_sudo(username: &str, password: &str) -> AuthResult {
    // First authenticate
    let result = authenticate(username, password);
    
    match &result {
        AuthResult::Success(user) | AuthResult::NoPasswordRequired(user) => {
            // Check if user is admin
            if user.is_admin {
                result
            } else {
                AuthResult::InvalidPassword  // Not authorized for sudo
            }
        }
        _ => result,
    }
}

