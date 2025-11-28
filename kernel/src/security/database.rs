//! User and Group Database
//!
//! Manages the user and group database, similar to /etc/passwd, /etc/group, /etc/shadow.
//!
//! ## Default Users
//! - `root` (uid=0): System administrator, disabled by default
//! - `debos` (uid=1000): Default user, admin, no password
//! - `nobody` (uid=65534): Unprivileged user
//!
//! ## Default Groups
//! - `root` (gid=0): System administrators
//! - `wheel` (gid=10): Sudo/admin access
//! - `users` (gid=100): Regular users
//! - `debos` (gid=1000): Default user group

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

use super::identity::{UserId, GroupId, User, Group, AccountStatus};
use super::auth::PasswordEntry;

lazy_static! {
    /// User database (username -> User)
    pub static ref USER_DB: Mutex<BTreeMap<String, User>> = Mutex::new(BTreeMap::new());
    
    /// User ID to username mapping
    pub static ref UID_MAP: Mutex<BTreeMap<u32, String>> = Mutex::new(BTreeMap::new());
    
    /// Group database (group name -> Group)
    pub static ref GROUP_DB: Mutex<BTreeMap<String, Group>> = Mutex::new(BTreeMap::new());
    
    /// Group ID to group name mapping
    pub static ref GID_MAP: Mutex<BTreeMap<u32, String>> = Mutex::new(BTreeMap::new());
    
    /// Password database (username -> PasswordEntry)
    pub static ref PASSWORD_DB: Mutex<BTreeMap<String, PasswordEntry>> = Mutex::new(BTreeMap::new());
    
    /// Next available user ID
    static ref NEXT_UID: Mutex<u32> = Mutex::new(1001);
    
    /// Next available group ID
    static ref NEXT_GID: Mutex<u32> = Mutex::new(1001);
}

/// Initialize the user database with default users and groups
pub fn init() {
    init_default_groups();
    init_default_users();
    init_default_passwords();
}

/// Initialize default groups
fn init_default_groups() {
    let mut groups = GROUP_DB.lock();
    let mut gid_map = GID_MAP.lock();
    
    // Root group
    let root_group = Group::root();
    gid_map.insert(root_group.gid.as_raw(), root_group.name.clone());
    groups.insert(root_group.name.clone(), root_group);
    
    // Wheel group (admin)
    let wheel_group = Group::wheel();
    gid_map.insert(wheel_group.gid.as_raw(), wheel_group.name.clone());
    groups.insert(wheel_group.name.clone(), wheel_group);
    
    // Users group
    let users_group = Group::users();
    gid_map.insert(users_group.gid.as_raw(), users_group.name.clone());
    groups.insert(users_group.name.clone(), users_group);
    
    // Debos user group
    let debos_group = Group::debos();
    gid_map.insert(debos_group.gid.as_raw(), debos_group.name.clone());
    groups.insert(debos_group.name.clone(), debos_group);
    
    // System groups
    add_system_group(&mut groups, &mut gid_map, 1, "daemon", "System Daemons");
    add_system_group(&mut groups, &mut gid_map, 2, "bin", "Binary Files");
    add_system_group(&mut groups, &mut gid_map, 3, "sys", "System Files");
    add_system_group(&mut groups, &mut gid_map, 4, "adm", "System Monitoring");
    add_system_group(&mut groups, &mut gid_map, 5, "tty", "Terminal Devices");
    add_system_group(&mut groups, &mut gid_map, 6, "disk", "Disk Devices");
    add_system_group(&mut groups, &mut gid_map, 20, "dialout", "Serial Ports");
    add_system_group(&mut groups, &mut gid_map, 27, "video", "Video Devices");
    add_system_group(&mut groups, &mut gid_map, 29, "audio", "Audio Devices");
}

fn add_system_group(
    groups: &mut BTreeMap<String, Group>,
    gid_map: &mut BTreeMap<u32, String>,
    gid: u32,
    name: &str,
    description: &str,
) {
    let group = Group::new(GroupId::new(gid), name, description);
    gid_map.insert(gid, String::from(name));
    groups.insert(String::from(name), group);
}

/// Initialize default users
fn init_default_users() {
    let mut users = USER_DB.lock();
    let mut uid_map = UID_MAP.lock();
    
    // Root user (disabled by default - no direct login)
    let mut root_user = User::root();
    root_user.status = AccountStatus::Disabled;  // Disable root login
    uid_map.insert(root_user.uid.as_raw(), root_user.username.clone());
    users.insert(root_user.username.clone(), root_user);
    
    // Debos user (default, admin, no password)
    let debos_user = User::debos();
    uid_map.insert(debos_user.uid.as_raw(), debos_user.username.clone());
    users.insert(debos_user.username.clone(), debos_user);
    
    // Nobody user
    let nobody_user = User::nobody();
    uid_map.insert(nobody_user.uid.as_raw(), nobody_user.username.clone());
    users.insert(nobody_user.username.clone(), nobody_user);
}

/// Initialize default password entries
fn init_default_passwords() {
    let mut passwords = PASSWORD_DB.lock();
    
    // Root has no password (login disabled anyway)
    passwords.insert(
        String::from("root"),
        PasswordEntry::no_password("root"),
    );
    
    // Debos user has no password (passwordless login)
    passwords.insert(
        String::from("debos"),
        PasswordEntry::no_password("debos"),
    );
    
    // Nobody has no password but can't login (shell is /bin/false)
    passwords.insert(
        String::from("nobody"),
        PasswordEntry::no_password("nobody"),
    );
}

// ============================================================================
// User Management
// ============================================================================

/// Get user by username
pub fn get_user_by_name(username: &str) -> Option<User> {
    USER_DB.lock().get(username).cloned()
}

/// Get user by UID
pub fn get_user_by_uid(uid: UserId) -> Option<User> {
    let uid_map = UID_MAP.lock();
    if let Some(username) = uid_map.get(&uid.as_raw()) {
        USER_DB.lock().get(username).cloned()
    } else {
        None
    }
}

/// Get username by UID
pub fn get_username(uid: UserId) -> Option<String> {
    UID_MAP.lock().get(&uid.as_raw()).cloned()
}

/// Create a new user
pub fn create_user(
    username: &str,
    full_name: &str,
    home_dir: Option<&str>,
    shell: Option<&str>,
    is_admin: bool,
    password: Option<&str>,
) -> Result<User, &'static str> {
    // Validate username
    if username.is_empty() || username.len() > 32 {
        return Err("Invalid username length");
    }
    
    if !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err("Username contains invalid characters");
    }
    
    // Check if user already exists
    if USER_DB.lock().contains_key(username) {
        return Err("User already exists");
    }
    
    // Allocate UID
    let uid = {
        let mut next_uid = NEXT_UID.lock();
        let uid = UserId::new(*next_uid);
        *next_uid += 1;
        uid
    };
    
    // Create user's primary group
    let gid = create_user_group(username)?;
    
    // Build home directory path
    let home = home_dir
        .map(String::from)
        .unwrap_or_else(|| alloc::format!("/home/{}", username));
    
    // Default shell
    let user_shell = shell.unwrap_or("/bin/sh");
    
    // Create user
    let mut user = User::new(uid, gid, username, full_name, &home, user_shell);
    
    // Add to groups
    user.groups.push(gid);
    user.groups.push(GroupId::USERS);
    
    if is_admin {
        user.groups.push(GroupId::WHEEL);
        user.is_admin = true;
        
        // Add to wheel group
        if let Some(wheel) = GROUP_DB.lock().get_mut("wheel") {
            wheel.add_member(uid);
        }
    }
    
    // Add to users group
    if let Some(users_group) = GROUP_DB.lock().get_mut("users") {
        users_group.add_member(uid);
    }
    
    // Store user
    {
        let mut users = USER_DB.lock();
        let mut uid_map = UID_MAP.lock();
        uid_map.insert(uid.as_raw(), String::from(username));
        users.insert(String::from(username), user.clone());
    }
    
    // Set password
    {
        let mut passwords = PASSWORD_DB.lock();
        let entry = if let Some(pw) = password {
            PasswordEntry::with_password(username, pw)
        } else {
            PasswordEntry::no_password(username)
        };
        passwords.insert(String::from(username), entry);
    }
    
    Ok(user)
}

/// Create a user's private group
fn create_user_group(username: &str) -> Result<GroupId, &'static str> {
    let gid = {
        let mut next_gid = NEXT_GID.lock();
        let gid = GroupId::new(*next_gid);
        *next_gid += 1;
        gid
    };
    
    let group = Group::new(gid, username, &alloc::format!("{}'s private group", username));
    
    let mut groups = GROUP_DB.lock();
    let mut gid_map = GID_MAP.lock();
    gid_map.insert(gid.as_raw(), String::from(username));
    groups.insert(String::from(username), group);
    
    Ok(gid)
}

/// Delete a user
pub fn delete_user(username: &str) -> Result<(), &'static str> {
    // Can't delete system users
    if username == "root" || username == "debos" || username == "nobody" {
        return Err("Cannot delete system user");
    }
    
    let uid = {
        let users = USER_DB.lock();
        users.get(username).map(|u| u.uid)
    };
    
    let uid = uid.ok_or("User not found")?;
    
    // Remove from all groups
    {
        let mut groups = GROUP_DB.lock();
        for group in groups.values_mut() {
            group.remove_member(uid);
        }
    }
    
    // Remove user's private group
    {
        let mut groups = GROUP_DB.lock();
        let mut gid_map = GID_MAP.lock();
        if let Some(group) = groups.remove(username) {
            gid_map.remove(&group.gid.as_raw());
        }
    }
    
    // Remove password entry
    PASSWORD_DB.lock().remove(username);
    
    // Remove user
    {
        let mut users = USER_DB.lock();
        let mut uid_map = UID_MAP.lock();
        uid_map.remove(&uid.as_raw());
        users.remove(username);
    }
    
    Ok(())
}

/// Modify user properties
pub fn modify_user(
    username: &str,
    new_full_name: Option<&str>,
    new_shell: Option<&str>,
    new_home: Option<&str>,
    add_admin: Option<bool>,
) -> Result<(), &'static str> {
    // First, get the user info and check if admin change is needed
    let (uid, was_admin, is_now_admin) = {
        let mut users = USER_DB.lock();
        let user = users.get_mut(username).ok_or("User not found")?;
        
        if let Some(name) = new_full_name {
            user.full_name = String::from(name);
        }
        
        if let Some(shell) = new_shell {
            user.shell = String::from(shell);
        }
        
        if let Some(home) = new_home {
            user.home_dir = String::from(home);
        }
        
        let was_admin = user.is_admin;
        
        if let Some(is_admin) = add_admin {
            if is_admin && !user.is_admin {
                user.add_to_group(GroupId::WHEEL);
            } else if !is_admin && user.is_admin {
                user.remove_from_group(GroupId::WHEEL);
            }
        }
        
        (user.uid, was_admin, user.is_admin)
    };
    
    // Now update the wheel group if needed (with users lock released)
    if is_now_admin && !was_admin {
        if let Some(wheel) = GROUP_DB.lock().get_mut("wheel") {
            wheel.add_member(uid);
        }
    } else if !is_now_admin && was_admin {
        if let Some(wheel) = GROUP_DB.lock().get_mut("wheel") {
            wheel.remove_member(uid);
        }
    }
    
    Ok(())
}

// ============================================================================
// Group Management
// ============================================================================

/// Get group by name
pub fn get_group_by_name(name: &str) -> Option<Group> {
    GROUP_DB.lock().get(name).cloned()
}

/// Get group by GID
pub fn get_group_by_gid(gid: GroupId) -> Option<Group> {
    let gid_map = GID_MAP.lock();
    if let Some(name) = gid_map.get(&gid.as_raw()) {
        GROUP_DB.lock().get(name).cloned()
    } else {
        None
    }
}

/// Get group name by GID
pub fn get_groupname(gid: GroupId) -> Option<String> {
    GID_MAP.lock().get(&gid.as_raw()).cloned()
}

/// Create a new group
pub fn create_group(name: &str, description: &str) -> Result<Group, &'static str> {
    if GROUP_DB.lock().contains_key(name) {
        return Err("Group already exists");
    }
    
    let gid = {
        let mut next_gid = NEXT_GID.lock();
        let gid = GroupId::new(*next_gid);
        *next_gid += 1;
        gid
    };
    
    let group = Group::new(gid, name, description);
    
    let mut groups = GROUP_DB.lock();
    let mut gid_map = GID_MAP.lock();
    gid_map.insert(gid.as_raw(), String::from(name));
    groups.insert(String::from(name), group.clone());
    
    Ok(group)
}

/// Delete a group
pub fn delete_group(name: &str) -> Result<(), &'static str> {
    // Can't delete system groups
    let protected = ["root", "wheel", "users", "daemon", "bin", "sys", "adm"];
    if protected.contains(&name) {
        return Err("Cannot delete system group");
    }
    
    let mut groups = GROUP_DB.lock();
    let mut gid_map = GID_MAP.lock();
    
    if let Some(group) = groups.remove(name) {
        gid_map.remove(&group.gid.as_raw());
        Ok(())
    } else {
        Err("Group not found")
    }
}

/// Add user to group
pub fn add_user_to_group(username: &str, groupname: &str) -> Result<(), &'static str> {
    let uid = {
        let users = USER_DB.lock();
        users.get(username).map(|u| u.uid)
    }.ok_or("User not found")?;
    
    let gid = {
        let groups = GROUP_DB.lock();
        groups.get(groupname).map(|g| g.gid)
    }.ok_or("Group not found")?;
    
    // Add to group
    {
        let mut groups = GROUP_DB.lock();
        if let Some(group) = groups.get_mut(groupname) {
            group.add_member(uid);
        }
    }
    
    // Update user's groups
    {
        let mut users = USER_DB.lock();
        if let Some(user) = users.get_mut(username) {
            user.add_to_group(gid);
        }
    }
    
    Ok(())
}

/// Remove user from group
pub fn remove_user_from_group(username: &str, groupname: &str) -> Result<(), &'static str> {
    let uid = {
        let users = USER_DB.lock();
        users.get(username).map(|u| u.uid)
    }.ok_or("User not found")?;
    
    let gid = {
        let groups = GROUP_DB.lock();
        groups.get(groupname).map(|g| g.gid)
    }.ok_or("Group not found")?;
    
    // Remove from group
    {
        let mut groups = GROUP_DB.lock();
        if let Some(group) = groups.get_mut(groupname) {
            group.remove_member(uid);
        }
    }
    
    // Update user's groups
    {
        let mut users = USER_DB.lock();
        if let Some(user) = users.get_mut(username) {
            user.remove_from_group(gid);
        }
    }
    
    Ok(())
}

/// List all users
pub fn list_users() -> Vec<User> {
    USER_DB.lock().values().cloned().collect()
}

/// List all groups
pub fn list_groups() -> Vec<Group> {
    GROUP_DB.lock().values().cloned().collect()
}

/// Get groups for a user
pub fn get_user_groups(username: &str) -> Vec<Group> {
    let users = USER_DB.lock();
    let groups = GROUP_DB.lock();
    
    if let Some(user) = users.get(username) {
        user.groups
            .iter()
            .filter_map(|gid| {
                let gid_map = GID_MAP.lock();
                gid_map.get(&gid.as_raw()).and_then(|name| groups.get(name).cloned())
            })
            .collect()
    } else {
        Vec::new()
    }
}

