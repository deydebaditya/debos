# DebOS User & Security System Implementation Plan

> **Phase 5: User Management, Authentication & Security**  
> **Goal:** Complete multi-user OS with enterprise-grade security  
> **Status:** Planning  
> **Prerequisites:** Phase 1 (Kernel), Phase 2A (Filesystem)

---

## Executive Summary

This document outlines the implementation of DebOS's user management and security subsystem. Unlike traditional UNIX systems, DebOS combines proven concepts with modern security practices:

- **Capability-based security** (not just UID checks)
- **Mandatory Access Control (MAC)** in addition to DAC
- **Cryptographically secure authentication**
- **Principle of least privilege by default**

### Design Goals

| Goal | Description | Priority |
|------|-------------|----------|
| **Multi-user isolation** | Complete process/memory isolation between users | Critical |
| **Defense in depth** | Multiple security layers, no single point of failure | Critical |
| **Least privilege** | Processes get minimum required permissions | Critical |
| **Auditability** | All security-relevant events logged | High |
| **Usability** | Security shouldn't impede normal use | Medium |
| **Performance** | Security checks < 1µs overhead | Medium |

---

## System Architecture

### Security Layers

```
┌─────────────────────────────────────────────────────────────────────┐
│                        APPLICATION LAYER                             │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────────┐  │
│  │ User Apps   │ │ System Apps │ │ Services    │ │ Shell         │  │
│  │ (uid=1000+) │ │ (uid=1-999) │ │ (dedicated) │ │ (login shell) │  │
│  └─────────────┘ └─────────────┘ └─────────────┘ └───────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     SECURITY POLICY LAYER                            │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                  Mandatory Access Control (MAC)                  ││
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────────┐ ││
│  │  │ Security     │ │ Domain       │ │ Integrity               │ ││
│  │  │ Labels       │ │ Transitions  │ │ Levels                  │ ││
│  │  └──────────────┘ └──────────────┘ └──────────────────────────┘ ││
│  └─────────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                Discretionary Access Control (DAC)                ││
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────────┐ ││
│  │  │ File         │ │ POSIX        │ │ Access Control          │ ││
│  │  │ Ownership    │ │ Permissions  │ │ Lists (ACLs)            │ ││
│  │  └──────────────┘ └──────────────┘ └──────────────────────────┘ ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     AUTHENTICATION LAYER                             │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────────┐  │
│  │ Password    │ │ Session     │ │ PAM-like    │ │ Credential    │  │
│  │ Hashing     │ │ Management  │ │ Modules     │ │ Store         │  │
│  │ (Argon2id)  │ │             │ │             │ │ (encrypted)   │  │
│  └─────────────┘ └─────────────┘ └─────────────┘ └───────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        KERNEL LAYER                                  │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                  Capability System (DeK)                         ││
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────────┐ ││
│  │  │ Capability   │ │ Capability   │ │ Capability              │ ││
│  │  │ Tokens       │ │ Derivation   │ │ Revocation              │ ││
│  │  └──────────────┘ └──────────────┘ └──────────────────────────┘ ││
│  └─────────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                     Process Credentials                          ││
│  │  UID │ GID │ Groups │ Capabilities │ Security Context           ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

---

## Component Designs

### 1. User Identity System

#### 1.1 User Account Structure

```rust
/// User account information
/// Stored in /etc/passwd equivalent
pub struct User {
    /// Unique user identifier (0 = root, 1-999 = system, 1000+ = regular)
    pub uid: UserId,
    
    /// Primary group identifier
    pub gid: GroupId,
    
    /// Username (1-32 chars, alphanumeric + underscore)
    pub username: String,
    
    /// Full name / GECOS field
    pub full_name: String,
    
    /// Home directory path
    pub home_dir: PathBuf,
    
    /// Login shell path
    pub shell: PathBuf,
    
    /// Account status
    pub status: AccountStatus,
    
    /// Account creation timestamp
    pub created_at: Timestamp,
    
    /// Last login timestamp
    pub last_login: Option<Timestamp>,
    
    /// Account expiration (optional)
    pub expires_at: Option<Timestamp>,
}

/// User identifier type
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(u32);

impl UserId {
    pub const ROOT: UserId = UserId(0);
    pub const NOBODY: UserId = UserId(65534);
    
    pub fn is_root(&self) -> bool { self.0 == 0 }
    pub fn is_system(&self) -> bool { self.0 > 0 && self.0 < 1000 }
    pub fn is_regular(&self) -> bool { self.0 >= 1000 }
}

/// Account status
pub enum AccountStatus {
    Active,
    Locked,
    Disabled,
    Expired,
}
```

#### 1.2 User ID Ranges

| UID Range | Type | Description |
|-----------|------|-------------|
| 0 | Root | Superuser, unrestricted access |
| 1-99 | Core System | Essential system services |
| 100-999 | System | System services and daemons |
| 1000-59999 | Regular Users | Normal user accounts |
| 60000-64999 | Reserved | Container/namespace users |
| 65534 | Nobody | Unprivileged placeholder |
| 65535 | Invalid | Reserved, never used |

#### 1.3 User Database

```
/etc/debos/
├── passwd          # User account info (world-readable)
├── shadow          # Password hashes (root-only)
├── group           # Group definitions
├── gshadow         # Group passwords (root-only)
├── login.defs      # Login defaults
└── security/
    ├── limits.conf     # Resource limits per user/group
    ├── access.conf     # Access rules
    └── policy.conf     # Security policies
```

### 2. Group System

#### 2.1 Group Structure

```rust
/// Group information
pub struct Group {
    /// Unique group identifier
    pub gid: GroupId,
    
    /// Group name
    pub name: String,
    
    /// Group description
    pub description: String,
    
    /// Member user IDs
    pub members: Vec<UserId>,
    
    /// Group administrators (can add/remove members)
    pub admins: Vec<UserId>,
}

/// Group identifier type
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupId(u32);

impl GroupId {
    pub const ROOT: GroupId = GroupId(0);
    pub const WHEEL: GroupId = GroupId(10);  // Sudo access
    pub const USERS: GroupId = GroupId(100);
    pub const NOGROUP: GroupId = GroupId(65534);
}
```

#### 2.2 Default Groups

| GID | Name | Purpose |
|-----|------|---------|
| 0 | root | Root group |
| 1 | daemon | System daemons |
| 2 | bin | Binary executables |
| 3 | sys | System files |
| 4 | adm | System monitoring |
| 5 | tty | Terminal devices |
| 6 | disk | Disk devices |
| 10 | wheel | Sudo/su access |
| 20 | dialout | Serial ports |
| 27 | video | Video devices |
| 29 | audio | Audio devices |
| 100 | users | Regular users |
| 1000+ | (user groups) | Per-user private groups |

#### 2.3 Group Membership Rules

```rust
/// Process group credentials
pub struct GroupCredentials {
    /// Primary group (from /etc/passwd)
    pub primary: GroupId,
    
    /// Supplementary groups (from /etc/group)
    pub supplementary: Vec<GroupId>,
    
    /// Maximum supplementary groups
    pub const MAX_GROUPS: usize = 32;
}

impl GroupCredentials {
    /// Check if process is member of group
    pub fn is_member(&self, gid: GroupId) -> bool {
        self.primary == gid || self.supplementary.contains(&gid)
    }
}
```

### 3. Authentication System

#### 3.1 Password Storage

```rust
/// Password hash entry (stored in /etc/shadow)
pub struct PasswordEntry {
    /// Username
    pub username: String,
    
    /// Password hash (Argon2id)
    pub hash: PasswordHash,
    
    /// Days since epoch of last password change
    pub last_change: u32,
    
    /// Minimum days between password changes
    pub min_age: u32,
    
    /// Maximum days before password must be changed
    pub max_age: u32,
    
    /// Days before expiry to warn user
    pub warn_days: u32,
    
    /// Days after expiry before account is disabled
    pub inactive_days: Option<u32>,
    
    /// Account expiration date (days since epoch)
    pub expire_date: Option<u32>,
}

/// Password hash using Argon2id
pub struct PasswordHash {
    /// Algorithm identifier
    pub algorithm: HashAlgorithm,
    
    /// Salt (16 bytes random)
    pub salt: [u8; 16],
    
    /// Hash output (32 bytes)
    pub hash: [u8; 32],
    
    /// Argon2 parameters
    pub params: Argon2Params,
}

/// Argon2id parameters
pub struct Argon2Params {
    /// Memory cost (KB)
    pub memory: u32,      // Default: 65536 (64 MB)
    
    /// Time cost (iterations)
    pub iterations: u32,  // Default: 3
    
    /// Parallelism
    pub parallelism: u32, // Default: 4
}
```

#### 3.2 Authentication Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                     LOGIN PROCESS                                    │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 1. IDENTIFICATION                                                    │
│    - Read username from input                                        │
│    - Look up user in /etc/passwd                                     │
│    - Check account status (active, locked, expired)                  │
│    ❌ Failure → Log attempt, delay, retry                           │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 2. AUTHENTICATION                                                    │
│    - Read password (no echo)                                         │
│    - Hash with Argon2id using stored salt                           │
│    - Compare with stored hash (constant-time)                        │
│    ❌ Failure → Log, increment fail count, delay exponentially      │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 3. AUTHORIZATION                                                     │
│    - Check login permissions (time, terminal, network)               │
│    - Load security context (MAC labels)                              │
│    - Check resource limits                                           │
│    ❌ Failure → Log, deny access                                    │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│ 4. SESSION CREATION                                                  │
│    - Fork new process                                                │
│    - Set UID, GID, supplementary groups                             │
│    - Drop capabilities (least privilege)                            │
│    - Change to home directory                                        │
│    - Execute login shell                                             │
│    ✅ Success → Log successful login                                │
└─────────────────────────────────────────────────────────────────────┘
```

#### 3.3 Authentication Modules (PAM-like)

```rust
/// Pluggable Authentication Module interface
pub trait AuthModule {
    /// Module name
    fn name(&self) -> &str;
    
    /// Authenticate user
    fn authenticate(&self, user: &str, credentials: &Credentials) 
        -> AuthResult;
    
    /// Check if user is authorized for this service
    fn authorize(&self, user: &User, service: &str) 
        -> AuthResult;
    
    /// Update credentials (e.g., password change)
    fn update_credentials(&self, user: &User, new_creds: &Credentials) 
        -> AuthResult;
    
    /// Session setup (called after successful auth)
    fn open_session(&self, user: &User) -> AuthResult;
    
    /// Session cleanup
    fn close_session(&self, user: &User) -> AuthResult;
}

/// Authentication result
pub enum AuthResult {
    Success,
    Failure(AuthError),
    Ignore,  // Module doesn't apply
    Die,     // Critical failure, stop immediately
}

/// Built-in authentication modules
pub enum BuiltinModules {
    Unix,       // Password-based (Argon2id)
    Deny,       // Always deny
    Permit,     // Always permit (dangerous!)
    Wheel,      // Require wheel group
    Time,       // Time-based restrictions
    Limits,     // Resource limits
    Securetty,  // Restrict root login to secure terminals
    Nologin,    // Check /etc/nologin
}
```

### 4. Process Credentials

#### 4.1 Credential Structure

```rust
/// Complete process credentials
pub struct ProcessCredentials {
    // === User/Group IDs ===
    
    /// Real user ID (who started the process)
    pub uid: UserId,
    
    /// Effective user ID (used for permission checks)
    pub euid: UserId,
    
    /// Saved user ID (for setuid programs)
    pub suid: UserId,
    
    /// Filesystem user ID (for filesystem access)
    pub fsuid: UserId,
    
    /// Real group ID
    pub gid: GroupId,
    
    /// Effective group ID
    pub egid: GroupId,
    
    /// Saved group ID
    pub sgid: GroupId,
    
    /// Filesystem group ID
    pub fsgid: GroupId,
    
    /// Supplementary groups
    pub groups: Vec<GroupId>,
    
    // === Capabilities ===
    
    /// Permitted capabilities (ceiling)
    pub cap_permitted: CapabilitySet,
    
    /// Effective capabilities (currently active)
    pub cap_effective: CapabilitySet,
    
    /// Inheritable capabilities (passed to exec'd programs)
    pub cap_inheritable: CapabilitySet,
    
    /// Bounding set (absolute limit)
    pub cap_bounding: CapabilitySet,
    
    // === Security Context ===
    
    /// MAC security label
    pub security_label: SecurityLabel,
    
    /// Integrity level
    pub integrity_level: IntegrityLevel,
    
    // === Session Info ===
    
    /// Session ID
    pub session_id: SessionId,
    
    /// Login timestamp
    pub login_time: Timestamp,
}
```

#### 4.2 Credential Transitions

```
┌─────────────────────────────────────────────────────────────────────┐
│                     SETUID EXECUTION                                 │
└─────────────────────────────────────────────────────────────────────┘

Before exec(/usr/bin/passwd):     After exec:
┌──────────────────────────┐      ┌──────────────────────────┐
│ uid  = 1000 (user)       │      │ uid  = 1000 (unchanged)  │
│ euid = 1000 (user)       │  →   │ euid = 0 (root!)         │
│ suid = 1000 (user)       │      │ suid = 0 (saved)         │
└──────────────────────────┘      └──────────────────────────┘

Security constraints:
- Setuid bit must be set on executable
- Executable must be owned by target user (root)
- Filesystem must be mounted without 'nosuid'
- Process must have CAP_SETUID capability
```

### 5. File Ownership & Permissions

#### 5.1 File Metadata

```rust
/// Extended file attributes for security
pub struct FileSecurityAttrs {
    // === POSIX Ownership ===
    
    /// Owner user ID
    pub uid: UserId,
    
    /// Owner group ID
    pub gid: GroupId,
    
    // === POSIX Permissions ===
    
    /// Permission bits (rwxrwxrwx)
    pub mode: FileMode,
    
    // === Special Bits ===
    
    /// Set-user-ID on execution
    pub setuid: bool,
    
    /// Set-group-ID on execution
    pub setgid: bool,
    
    /// Sticky bit (restricted deletion)
    pub sticky: bool,
    
    // === Extended Attributes ===
    
    /// Access Control List (optional)
    pub acl: Option<AccessControlList>,
    
    /// MAC security label
    pub security_label: Option<SecurityLabel>,
    
    /// Immutable flag (cannot be modified)
    pub immutable: bool,
    
    /// Append-only flag
    pub append_only: bool,
}

/// POSIX file mode
#[derive(Clone, Copy)]
pub struct FileMode(u16);

impl FileMode {
    // Owner permissions
    pub const S_IRUSR: u16 = 0o400;  // Owner read
    pub const S_IWUSR: u16 = 0o200;  // Owner write
    pub const S_IXUSR: u16 = 0o100;  // Owner execute
    
    // Group permissions
    pub const S_IRGRP: u16 = 0o040;  // Group read
    pub const S_IWGRP: u16 = 0o020;  // Group write
    pub const S_IXGRP: u16 = 0o010;  // Group execute
    
    // Other permissions
    pub const S_IROTH: u16 = 0o004;  // Other read
    pub const S_IWOTH: u16 = 0o002;  // Other write
    pub const S_IXOTH: u16 = 0o001;  // Other execute
    
    // Special bits
    pub const S_ISUID: u16 = 0o4000; // Set-user-ID
    pub const S_ISGID: u16 = 0o2000; // Set-group-ID
    pub const S_ISVTX: u16 = 0o1000; // Sticky bit
    
    // Common combinations
    pub const DEFAULT_FILE: u16 = 0o644;      // rw-r--r--
    pub const DEFAULT_DIR: u16 = 0o755;       // rwxr-xr-x
    pub const PRIVATE_FILE: u16 = 0o600;      // rw-------
    pub const EXECUTABLE: u16 = 0o755;        // rwxr-xr-x
}
```

#### 5.2 Permission Checking Algorithm

```rust
/// Check if process can access file
pub fn check_access(
    process: &ProcessCredentials,
    file: &FileSecurityAttrs,
    requested: AccessMode,
) -> AccessResult {
    // 1. Root bypass (unless capability disabled)
    if process.euid.is_root() && process.cap_effective.contains(CAP_DAC_OVERRIDE) {
        // Root can read/write anything, but execute needs at least one x bit
        if requested.contains(AccessMode::EXECUTE) {
            if !file.mode.has_any_execute() {
                return AccessResult::Denied(EACCES);
            }
        }
        return AccessResult::Allowed;
    }
    
    // 2. Check MAC policy first (if enabled)
    if let Some(ref label) = file.security_label {
        if !mac_check(process.security_label, label, requested) {
            return AccessResult::Denied(EACCES);
        }
    }
    
    // 3. Check ACL (if present)
    if let Some(ref acl) = file.acl {
        if let Some(result) = acl.check(process, requested) {
            return result;
        }
    }
    
    // 4. Standard POSIX permission check
    let mode = if process.euid == file.uid {
        // Owner permissions
        (file.mode.0 >> 6) & 0o7
    } else if process.is_member(file.gid) {
        // Group permissions
        (file.mode.0 >> 3) & 0o7
    } else {
        // Other permissions
        file.mode.0 & 0o7
    };
    
    // Check each requested permission
    if requested.contains(AccessMode::READ) && (mode & 0o4) == 0 {
        return AccessResult::Denied(EACCES);
    }
    if requested.contains(AccessMode::WRITE) && (mode & 0o2) == 0 {
        return AccessResult::Denied(EACCES);
    }
    if requested.contains(AccessMode::EXECUTE) && (mode & 0o1) == 0 {
        return AccessResult::Denied(EACCES);
    }
    
    AccessResult::Allowed
}
```

### 6. Superuser (Root) Model

#### 6.1 Root Capabilities

DebOS uses a capability-based model where root's power can be decomposed:

```rust
/// System capabilities (Linux-compatible + DebOS extensions)
pub enum Capability {
    // === Standard Capabilities ===
    
    /// Override file read/write permissions
    CAP_DAC_OVERRIDE = 0,
    
    /// Override file read permissions
    CAP_DAC_READ_SEARCH = 1,
    
    /// Bypass file ownership checks
    CAP_FOWNER = 2,
    
    /// Don't clear setuid/setgid on modification
    CAP_FSETID = 3,
    
    /// Allow killing any process
    CAP_KILL = 4,
    
    /// Change process UIDs
    CAP_SETUID = 5,
    
    /// Change process GIDs
    CAP_SETGID = 6,
    
    /// Manipulate process capabilities
    CAP_SETPCAP = 7,
    
    /// Bind to ports < 1024
    CAP_NET_BIND_SERVICE = 8,
    
    /// Allow raw sockets
    CAP_NET_RAW = 9,
    
    /// Change file ownership
    CAP_CHOWN = 10,
    
    /// Lock memory (mlock)
    CAP_IPC_LOCK = 11,
    
    /// Load kernel modules
    CAP_SYS_MODULE = 12,
    
    /// Access raw I/O ports
    CAP_SYS_RAWIO = 13,
    
    /// Change root directory
    CAP_SYS_CHROOT = 14,
    
    /// Trace any process
    CAP_SYS_PTRACE = 15,
    
    /// Set system time
    CAP_SYS_TIME = 16,
    
    /// Administrate syslog
    CAP_SYSLOG = 17,
    
    // === DebOS Extensions ===
    
    /// Access kernel debugging
    CAP_DEBOS_DEBUG = 32,
    
    /// Modify security policy
    CAP_DEBOS_POLICY = 33,
    
    /// Access hardware directly (still via kernel)
    CAP_DEBOS_HARDWARE = 34,
    
    /// Manage system services
    CAP_DEBOS_SERVICE = 35,
}

/// Capability set (bitmap)
pub struct CapabilitySet(u64);

impl CapabilitySet {
    /// All capabilities (full root)
    pub const ALL: CapabilitySet = CapabilitySet(!0);
    
    /// No capabilities
    pub const EMPTY: CapabilitySet = CapabilitySet(0);
    
    /// Default for regular users
    pub const USER_DEFAULT: CapabilitySet = CapabilitySet(0);
    
    /// Network service (bind to privileged ports)
    pub const NET_SERVICE: CapabilitySet = CapabilitySet(
        (1 << CAP_NET_BIND_SERVICE) | (1 << CAP_NET_RAW)
    );
}
```

#### 6.2 Privilege Escalation Paths

```
┌─────────────────────────────────────────────────────────────────────┐
│                     CONTROLLED PRIVILEGE ESCALATION                  │
└─────────────────────────────────────────────────────────────────────┘

1. SETUID EXECUTABLES (e.g., /usr/bin/passwd)
   ┌─────────────────────────────────────────────────────────────────┐
   │ User runs 'passwd'                                              │
   │   → Executable has setuid bit + owned by root                   │
   │   → Process euid becomes 0                                      │
   │   → Capabilities limited to what's needed (CAP_CHOWN, etc.)     │
   │   → After operation, privileges dropped                         │
   └─────────────────────────────────────────────────────────────────┘

2. SUDO/SU (Controlled root access)
   ┌─────────────────────────────────────────────────────────────────┐
   │ User runs 'sudo command'                                        │
   │   → Check user is in 'wheel' group                              │
   │   → Authenticate user (re-enter password)                       │
   │   → Check /etc/sudoers policy                                   │
   │   → Fork with root credentials                                  │
   │   → Log action for audit                                        │
   └─────────────────────────────────────────────────────────────────┘

3. CAPABILITY GRANTS (Fine-grained)
   ┌─────────────────────────────────────────────────────────────────┐
   │ Service needs specific capability                               │
   │   → Configure in service definition                             │
   │   → Service manager grants specific capability                  │
   │   → NO root access, only specific permission                    │
   │   → Example: Web server gets CAP_NET_BIND_SERVICE only          │
   └─────────────────────────────────────────────────────────────────┘
```

#### 6.3 Root Login Restrictions

```rust
/// Root login policy
pub struct RootLoginPolicy {
    /// Allow root login at all
    pub enabled: bool,
    
    /// Terminals where root can login
    pub allowed_ttys: Vec<String>,  // e.g., ["tty1", "ttyS0"]
    
    /// Allow root login via SSH
    pub allow_ssh: bool,
    
    /// Require additional factor for root
    pub require_mfa: bool,
    
    /// Time restrictions (e.g., business hours only)
    pub time_restrictions: Option<TimePolicy>,
    
    /// Maximum failed attempts before lockout
    pub max_failed_attempts: u32,
    
    /// Lockout duration after max failures
    pub lockout_duration: Duration,
}
```

### 7. Security Policies

#### 7.1 Mandatory Access Control (MAC)

```rust
/// Security label for MAC
pub struct SecurityLabel {
    /// Sensitivity level (e.g., public, confidential, secret)
    pub sensitivity: SensitivityLevel,
    
    /// Integrity level (e.g., untrusted, user, system, trusted)
    pub integrity: IntegrityLevel,
    
    /// Category set (compartments)
    pub categories: CategorySet,
    
    /// Domain type (for type enforcement)
    pub domain: DomainType,
}

/// Sensitivity levels (Bell-LaPadula model)
#[derive(PartialOrd, Ord, PartialEq, Eq)]
pub enum SensitivityLevel {
    Public = 0,
    Internal = 1,
    Confidential = 2,
    Secret = 3,
    TopSecret = 4,
}

/// Integrity levels (Biba model)
#[derive(PartialOrd, Ord, PartialEq, Eq)]
pub enum IntegrityLevel {
    Untrusted = 0,   // Downloaded files, user data
    User = 1,        // User-created content
    System = 2,      // System configuration
    Trusted = 3,     // Kernel, core system
}

/// MAC access rules
impl SecurityLabel {
    /// Can this subject read this object? (Bell-LaPadula: no read up)
    pub fn can_read(&self, object: &SecurityLabel) -> bool {
        self.sensitivity >= object.sensitivity
            && self.categories.is_superset(&object.categories)
    }
    
    /// Can this subject write this object? (Bell-LaPadula: no write down)
    pub fn can_write(&self, object: &SecurityLabel) -> bool {
        self.sensitivity <= object.sensitivity
            && object.categories.is_superset(&self.categories)
    }
    
    /// Integrity check (Biba: no write up, no read down)
    pub fn integrity_check(&self, object: &SecurityLabel, write: bool) -> bool {
        if write {
            // Can't write to higher integrity
            self.integrity >= object.integrity
        } else {
            // Can't read from lower integrity
            self.integrity <= object.integrity
        }
    }
}
```

#### 7.2 Resource Limits

```rust
/// Per-user/group resource limits
pub struct ResourceLimits {
    /// Maximum number of processes
    pub max_processes: u32,
    
    /// Maximum open files
    pub max_open_files: u32,
    
    /// Maximum file size (bytes)
    pub max_file_size: u64,
    
    /// Maximum memory (bytes)
    pub max_memory: u64,
    
    /// Maximum CPU time (seconds)
    pub max_cpu_time: u64,
    
    /// Maximum threads per process
    pub max_threads: u32,
    
    /// Maximum pending signals
    pub max_signals: u32,
    
    /// Maximum message queue size
    pub max_msgqueue: u64,
    
    /// Nice priority range
    pub nice_priority: (i8, i8),  // min, max
    
    /// Real-time priority range (0 = not allowed)
    pub rt_priority: (u8, u8),
}

/// Default limits
impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_processes: 1024,
            max_open_files: 1024,
            max_file_size: 10 * 1024 * 1024 * 1024,  // 10 GB
            max_memory: 8 * 1024 * 1024 * 1024,       // 8 GB
            max_cpu_time: u64::MAX,                   // Unlimited
            max_threads: 256,
            max_signals: 64,
            max_msgqueue: 8 * 1024 * 1024,            // 8 MB
            nice_priority: (0, 19),                   // Can only lower priority
            rt_priority: (0, 0),                      // No real-time by default
        }
    }
}
```

#### 7.3 Audit Logging

```rust
/// Security audit event
pub struct AuditEvent {
    /// Timestamp
    pub timestamp: Timestamp,
    
    /// Event type
    pub event_type: AuditEventType,
    
    /// Subject (who)
    pub subject: AuditSubject,
    
    /// Object (what)
    pub object: Option<AuditObject>,
    
    /// Action taken
    pub action: String,
    
    /// Result
    pub result: AuditResult,
    
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Auditable events
pub enum AuditEventType {
    // Authentication
    LoginSuccess,
    LoginFailure,
    Logout,
    PasswordChange,
    
    // Authorization
    PermissionDenied,
    CapabilityUsed,
    PrivilegeEscalation,
    
    // File operations
    FileAccess,
    FileModify,
    FileDelete,
    OwnershipChange,
    PermissionChange,
    
    // Process operations
    ProcessCreate,
    ProcessTerminate,
    SignalSent,
    
    // System operations
    SystemBoot,
    SystemShutdown,
    ConfigChange,
    PolicyChange,
}
```

---

## Implementation Phases

### Phase 5A: Core Identity System (1 week)

| Task | Description | Priority |
|------|-------------|----------|
| USER-001 | UserId and GroupId types | Critical |
| USER-002 | User struct and serialization | Critical |
| USER-003 | Group struct and membership | Critical |
| USER-004 | /etc/passwd and /etc/group parsers | Critical |
| USER-005 | User database manager | Critical |

### Phase 5B: Process Credentials (1 week)

| Task | Description | Priority |
|------|-------------|----------|
| CRED-001 | ProcessCredentials struct | Critical |
| CRED-002 | setuid/setgid syscalls | Critical |
| CRED-003 | Credential inheritance on fork | Critical |
| CRED-004 | Credential transition on exec | Critical |
| CRED-005 | Supplementary groups handling | Critical |

### Phase 5C: Authentication System (2 weeks)

| Task | Description | Priority |
|------|-------------|----------|
| AUTH-001 | Password hashing (Argon2id) | Critical |
| AUTH-002 | /etc/shadow parser/writer | Critical |
| AUTH-003 | Login program | Critical |
| AUTH-004 | Session management | Critical |
| AUTH-005 | PAM-like module interface | High |
| AUTH-006 | Failed login handling | High |
| AUTH-007 | Account lockout | High |

### Phase 5D: File Permissions (1 week)

| Task | Description | Priority |
|------|-------------|----------|
| PERM-001 | File mode in inode | Critical |
| PERM-002 | chown/chmod syscalls | Critical |
| PERM-003 | Permission checking | Critical |
| PERM-004 | Setuid/setgid bit handling | Critical |
| PERM-005 | umask support | High |

### Phase 5E: Capability System (2 weeks)

| Task | Description | Priority |
|------|-------------|----------|
| CAP-001 | CapabilitySet type | Critical |
| CAP-002 | Per-process capability sets | Critical |
| CAP-003 | Capability inheritance rules | Critical |
| CAP-004 | Capability bounding set | Critical |
| CAP-005 | File capabilities | High |
| CAP-006 | Capability-aware syscalls | High |

### Phase 5F: Security Policies (2 weeks)

| Task | Description | Priority |
|------|-------------|----------|
| POL-001 | Resource limits | High |
| POL-002 | Security labels (MAC) | Medium |
| POL-003 | Audit logging | High |
| POL-004 | Login restrictions | High |
| POL-005 | Access control lists (ACL) | Medium |

### Phase 5G: User Management Commands (1 week)

| Task | Description | Priority |
|------|-------------|----------|
| CMD-001 | useradd command | Critical |
| CMD-002 | userdel command | Critical |
| CMD-003 | usermod command | High |
| CMD-004 | passwd command | Critical |
| CMD-005 | groupadd/groupdel/groupmod | High |
| CMD-006 | su command | Critical |
| CMD-007 | sudo command | Critical |
| CMD-008 | id/whoami/groups commands | High |

---

## Security Considerations

### 1. Password Security

| Measure | Implementation |
|---------|---------------|
| Strong hashing | Argon2id with 64MB memory, 3 iterations |
| Salted hashes | 128-bit random salt per password |
| Constant-time comparison | Prevent timing attacks |
| No plaintext storage | Never store or log passwords |
| Password policies | Minimum length, complexity, expiration |

### 2. Defense Against Common Attacks

| Attack | Defense |
|--------|---------|
| Brute force | Exponential backoff, account lockout |
| Privilege escalation | Capability-based, least privilege |
| Buffer overflow | Rust memory safety, stack canaries |
| Race conditions | Careful credential handling, atomics |
| Side channels | Constant-time operations |
| Path traversal | Strict path validation |

### 3. Audit Trail

All security-relevant operations are logged:
- Login attempts (success and failure)
- Privilege escalation
- File permission changes
- Capability usage
- Policy changes

---

## Shell Commands Summary

| Command | Description | Requires |
|---------|-------------|----------|
| `login` | Authenticate and start session | - |
| `logout` | End current session | - |
| `passwd` | Change password | setuid root |
| `su` | Switch user | setuid root |
| `sudo` | Execute as another user | setuid root |
| `useradd` | Create user | root |
| `userdel` | Delete user | root |
| `usermod` | Modify user | root |
| `groupadd` | Create group | root |
| `groupdel` | Delete group | root |
| `groupmod` | Modify group | root |
| `chown` | Change file owner | root or owner |
| `chmod` | Change file permissions | root or owner |
| `chgrp` | Change file group | root or owner |
| `id` | Display user/group info | - |
| `whoami` | Display current username | - |
| `groups` | Display group memberships | - |

---

## File System Layout

```
/etc/debos/
├── passwd              # User accounts (world-readable)
├── shadow              # Password hashes (root-only, 0600)
├── group               # Group definitions (world-readable)
├── gshadow             # Group passwords (root-only, 0600)
├── login.defs          # Login configuration
├── shells              # Valid login shells
├── securetty           # Terminals allowing root login
├── nologin             # If exists, non-root login disabled
├── sudoers             # Sudo configuration (root-only, 0440)
├── sudoers.d/          # Drop-in sudo configs
└── security/
    ├── limits.conf     # Resource limits
    ├── access.conf     # Access rules
    ├── policy.conf     # Security policy
    ├── capability.conf # Default capabilities
    └── audit.conf      # Audit configuration

/var/log/
├── auth.log            # Authentication logs
├── secure              # Security events
└── audit/
    └── audit.log       # Detailed audit trail
```

---

## Success Criteria

| Criterion | Test |
|-----------|------|
| User isolation | Process A (uid=1000) cannot access Process B (uid=1001) memory or files |
| Authentication | Invalid passwords rejected, valid passwords accepted |
| File permissions | Permission denied when accessing files without proper permissions |
| Root restrictions | Root can be restricted to specific terminals |
| Capabilities | Non-root process with specific capability can perform that action |
| Audit | All login attempts visible in logs |
| Password security | Passwords stored using Argon2id, not recoverable |

---

*This document will be updated as implementation progresses.*

