//! User and Group Identity Types
//!
//! Defines the core identity primitives for DebOS security.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

// ============================================================================
// User ID
// ============================================================================

/// User identifier
/// 
/// ## ID Ranges
/// - 0: Root (superuser)
/// - 1-999: System users (daemons, services)
/// - 1000-59999: Regular users
/// - 60000-64999: Reserved (containers, namespaces)
/// - 65534: Nobody (unprivileged)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(pub u32);

impl UserId {
    /// Root user (superuser)
    pub const ROOT: UserId = UserId(0);
    
    /// Nobody user (unprivileged placeholder)
    pub const NOBODY: UserId = UserId(65534);
    
    /// Default user (debos)
    pub const DEBOS: UserId = UserId(1000);
    
    /// First regular user ID
    pub const FIRST_REGULAR: u32 = 1000;
    
    /// Last regular user ID
    pub const LAST_REGULAR: u32 = 59999;
    
    /// Create a new user ID
    pub const fn new(id: u32) -> Self {
        UserId(id)
    }
    
    /// Check if this is the root user
    pub fn is_root(&self) -> bool {
        self.0 == 0
    }
    
    /// Check if this is a system user (1-999)
    pub fn is_system(&self) -> bool {
        self.0 > 0 && self.0 < 1000
    }
    
    /// Check if this is a regular user (1000+)
    pub fn is_regular(&self) -> bool {
        self.0 >= 1000 && self.0 < 65534
    }
    
    /// Get the raw ID value
    pub fn as_raw(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UserId({})", self.0)
    }
}

impl From<u32> for UserId {
    fn from(id: u32) -> Self {
        UserId(id)
    }
}

// ============================================================================
// Group ID
// ============================================================================

/// Group identifier
///
/// ## Default Groups
/// - 0: root
/// - 10: wheel (admin/sudo access)
/// - 100: users (regular users)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupId(pub u32);

impl GroupId {
    /// Root group
    pub const ROOT: GroupId = GroupId(0);
    
    /// Wheel group (admin access, sudo)
    pub const WHEEL: GroupId = GroupId(10);
    
    /// Users group (all regular users)
    pub const USERS: GroupId = GroupId(100);
    
    /// Nogroup (unprivileged)
    pub const NOGROUP: GroupId = GroupId(65534);
    
    /// Default user group (debos)
    pub const DEBOS: GroupId = GroupId(1000);
    
    /// Create a new group ID
    pub const fn new(id: u32) -> Self {
        GroupId(id)
    }
    
    /// Check if this is the root group
    pub fn is_root(&self) -> bool {
        self.0 == 0
    }
    
    /// Check if this is the wheel (admin) group
    pub fn is_wheel(&self) -> bool {
        self.0 == 10
    }
    
    /// Get the raw ID value
    pub fn as_raw(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GroupId({})", self.0)
    }
}

impl From<u32> for GroupId {
    fn from(id: u32) -> Self {
        GroupId(id)
    }
}

// ============================================================================
// User Account
// ============================================================================

/// Account status
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AccountStatus {
    /// Account is active and can log in
    Active,
    /// Account is locked (too many failed attempts)
    Locked,
    /// Account is disabled by admin
    Disabled,
    /// Account has expired
    Expired,
}

impl Default for AccountStatus {
    fn default() -> Self {
        AccountStatus::Active
    }
}

/// User account information
#[derive(Clone)]
pub struct User {
    /// User ID
    pub uid: UserId,
    
    /// Primary group ID
    pub gid: GroupId,
    
    /// Username (1-32 chars)
    pub username: String,
    
    /// Full name (GECOS field)
    pub full_name: String,
    
    /// Home directory path
    pub home_dir: String,
    
    /// Login shell path
    pub shell: String,
    
    /// Account status
    pub status: AccountStatus,
    
    /// Is this user an administrator (member of wheel)?
    pub is_admin: bool,
    
    /// Supplementary group memberships
    pub groups: Vec<GroupId>,
}

impl User {
    /// Create a new user
    pub fn new(
        uid: UserId,
        gid: GroupId,
        username: &str,
        full_name: &str,
        home_dir: &str,
        shell: &str,
    ) -> Self {
        User {
            uid,
            gid,
            username: String::from(username),
            full_name: String::from(full_name),
            home_dir: String::from(home_dir),
            shell: String::from(shell),
            status: AccountStatus::Active,
            is_admin: false,
            groups: Vec::new(),
        }
    }
    
    /// Create the root user
    pub fn root() -> Self {
        User {
            uid: UserId::ROOT,
            gid: GroupId::ROOT,
            username: String::from("root"),
            full_name: String::from("System Administrator"),
            home_dir: String::from("/root"),
            shell: String::from("/bin/sh"),
            status: AccountStatus::Active,
            is_admin: true,
            groups: alloc::vec![GroupId::ROOT, GroupId::WHEEL],
        }
    }
    
    /// Create the default debos user
    pub fn debos() -> Self {
        User {
            uid: UserId::DEBOS,
            gid: GroupId::DEBOS,
            username: String::from("debos"),
            full_name: String::from("DebOS Default User"),
            home_dir: String::from("/home/debos"),
            shell: String::from("/bin/sh"),
            status: AccountStatus::Active,
            is_admin: true,  // Default user is admin
            groups: alloc::vec![GroupId::DEBOS, GroupId::WHEEL, GroupId::USERS],
        }
    }
    
    /// Create the nobody user
    pub fn nobody() -> Self {
        User {
            uid: UserId::NOBODY,
            gid: GroupId::NOGROUP,
            username: String::from("nobody"),
            full_name: String::from("Unprivileged User"),
            home_dir: String::from("/nonexistent"),
            shell: String::from("/bin/false"),
            status: AccountStatus::Active,
            is_admin: false,
            groups: Vec::new(),
        }
    }
    
    /// Check if user is member of a group
    pub fn is_member_of(&self, gid: GroupId) -> bool {
        self.gid == gid || self.groups.contains(&gid)
    }
    
    /// Add user to a group
    pub fn add_to_group(&mut self, gid: GroupId) {
        if !self.groups.contains(&gid) {
            self.groups.push(gid);
            if gid == GroupId::WHEEL {
                self.is_admin = true;
            }
        }
    }
    
    /// Remove user from a group
    pub fn remove_from_group(&mut self, gid: GroupId) {
        self.groups.retain(|&g| g != gid);
        if gid == GroupId::WHEEL {
            self.is_admin = false;
        }
    }
}

impl fmt::Debug for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("User")
            .field("uid", &self.uid)
            .field("gid", &self.gid)
            .field("username", &self.username)
            .field("is_admin", &self.is_admin)
            .finish()
    }
}

// ============================================================================
// Group
// ============================================================================

/// Group information
#[derive(Clone)]
pub struct Group {
    /// Group ID
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

impl Group {
    /// Create a new group
    pub fn new(gid: GroupId, name: &str, description: &str) -> Self {
        Group {
            gid,
            name: String::from(name),
            description: String::from(description),
            members: Vec::new(),
            admins: Vec::new(),
        }
    }
    
    /// Create the root group
    pub fn root() -> Self {
        Group {
            gid: GroupId::ROOT,
            name: String::from("root"),
            description: String::from("System Administrators"),
            members: alloc::vec![UserId::ROOT],
            admins: alloc::vec![UserId::ROOT],
        }
    }
    
    /// Create the wheel (admin) group
    pub fn wheel() -> Self {
        Group {
            gid: GroupId::WHEEL,
            name: String::from("wheel"),
            description: String::from("Administrators (sudo access)"),
            members: alloc::vec![UserId::ROOT, UserId::DEBOS],
            admins: alloc::vec![UserId::ROOT],
        }
    }
    
    /// Create the users group
    pub fn users() -> Self {
        Group {
            gid: GroupId::USERS,
            name: String::from("users"),
            description: String::from("Regular Users"),
            members: alloc::vec![UserId::DEBOS],
            admins: alloc::vec![UserId::ROOT],
        }
    }
    
    /// Create the debos user group
    pub fn debos() -> Self {
        Group {
            gid: GroupId::DEBOS,
            name: String::from("debos"),
            description: String::from("DebOS Default User Group"),
            members: alloc::vec![UserId::DEBOS],
            admins: alloc::vec![UserId::ROOT, UserId::DEBOS],
        }
    }
    
    /// Check if user is a member
    pub fn has_member(&self, uid: UserId) -> bool {
        self.members.contains(&uid)
    }
    
    /// Add a member
    pub fn add_member(&mut self, uid: UserId) {
        if !self.members.contains(&uid) {
            self.members.push(uid);
        }
    }
    
    /// Remove a member
    pub fn remove_member(&mut self, uid: UserId) {
        self.members.retain(|&u| u != uid);
    }
    
    /// Check if user is an admin of this group
    pub fn is_admin(&self, uid: UserId) -> bool {
        self.admins.contains(&uid) || uid == UserId::ROOT
    }
}

impl fmt::Debug for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Group")
            .field("gid", &self.gid)
            .field("name", &self.name)
            .field("members", &self.members.len())
            .finish()
    }
}

