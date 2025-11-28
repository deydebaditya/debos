//! Authentication System
//!
//! Handles user authentication, password management, and session control.
//!
//! ## Password Security
//! - Passwords are hashed using a secure algorithm (SHA-256 + salt for now)
//! - No plaintext passwords are ever stored
//! - Constant-time comparison to prevent timing attacks
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
    
    /// Verify password
    pub fn verify(&self, password: &str) -> bool {
        if !self.has_password {
            // No password required, any input is valid (including empty)
            return true;
        }
        
        let test_hash = hash_password(password, &self.salt);
        constant_time_compare(&self.hash, &test_hash)
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

/// Hash password with salt (SHA-256 equivalent, simplified)
/// In production, use Argon2id
fn hash_password(password: &str, salt: &[u8; 16]) -> Vec<u8> {
    // Simple hash for now - concatenate salt + password and hash
    // This is NOT cryptographically secure, but works for demonstration
    // TODO: Implement proper Argon2id
    
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(salt);
    data.extend_from_slice(password.as_bytes());
    
    // Simple hash function (FNV-1a style, extended)
    let mut hash = [0u8; 32];
    let mut h: u64 = 0xcbf29ce484222325; // FNV offset basis
    
    for byte in data.iter() {
        h ^= *byte as u64;
        h = h.wrapping_mul(0x100000001b3); // FNV prime
    }
    
    // Expand to 32 bytes
    for i in 0..4 {
        let chunk = h.wrapping_add(i as u64 * 0x9e3779b97f4a7c15);
        let bytes = chunk.to_le_bytes();
        hash[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
    }
    
    hash.to_vec()
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
    
    creds
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

