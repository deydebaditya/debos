//! Process Credentials
//!
//! Each process has credentials that determine its identity and permissions.
//!
//! ## Credential Types
//! - **Real UID/GID**: Who started the process
//! - **Effective UID/GID**: Used for permission checks
//! - **Saved UID/GID**: For setuid programs to switch back
//! - **Filesystem UID/GID**: For file access checks

use alloc::vec::Vec;
use super::identity::{UserId, GroupId};
use super::capability::CapabilitySet;

/// Maximum number of supplementary groups
pub const MAX_GROUPS: usize = 32;

/// Process credentials
#[derive(Clone)]
pub struct ProcessCredentials {
    // === User IDs ===
    
    /// Real user ID (who started the process)
    pub uid: UserId,
    
    /// Effective user ID (used for permission checks)
    pub euid: UserId,
    
    /// Saved user ID (for setuid programs)
    pub suid: UserId,
    
    /// Filesystem user ID (for file access)
    pub fsuid: UserId,
    
    // === Group IDs ===
    
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
    
    /// Effective capabilities (currently active)
    pub capabilities: CapabilitySet,
    
    // === Session Info ===
    
    /// Session ID
    pub session_id: u32,
    
    /// Is this an administrator (member of wheel)?
    pub is_admin: bool,
}

impl ProcessCredentials {
    /// Create credentials for the kernel (root with all capabilities)
    pub fn kernel() -> Self {
        ProcessCredentials {
            uid: UserId::ROOT,
            euid: UserId::ROOT,
            suid: UserId::ROOT,
            fsuid: UserId::ROOT,
            gid: GroupId::ROOT,
            egid: GroupId::ROOT,
            sgid: GroupId::ROOT,
            fsgid: GroupId::ROOT,
            groups: alloc::vec![GroupId::ROOT, GroupId::WHEEL],
            capabilities: CapabilitySet::all(),
            session_id: 0,
            is_admin: true,
        }
    }
    
    /// Create credentials for root user
    pub fn root() -> Self {
        Self::kernel()
    }
    
    /// Create credentials for the default debos user
    pub fn debos() -> Self {
        ProcessCredentials {
            uid: UserId::DEBOS,
            euid: UserId::DEBOS,
            suid: UserId::DEBOS,
            fsuid: UserId::DEBOS,
            gid: GroupId::DEBOS,
            egid: GroupId::DEBOS,
            sgid: GroupId::DEBOS,
            fsgid: GroupId::DEBOS,
            groups: alloc::vec![GroupId::DEBOS, GroupId::WHEEL, GroupId::USERS],
            capabilities: CapabilitySet::user_default(),
            session_id: 1,
            is_admin: true,
        }
    }
    
    /// Create credentials for a regular user
    pub fn for_user(uid: UserId, gid: GroupId, groups: Vec<GroupId>, is_admin: bool) -> Self {
        let caps = if is_admin {
            CapabilitySet::admin_default()
        } else {
            CapabilitySet::user_default()
        };
        
        ProcessCredentials {
            uid,
            euid: uid,
            suid: uid,
            fsuid: uid,
            gid,
            egid: gid,
            sgid: gid,
            fsgid: gid,
            groups,
            capabilities: caps,
            session_id: 0,
            is_admin,
        }
    }
    
    /// Check if running as root
    pub fn is_root(&self) -> bool {
        self.euid == UserId::ROOT
    }
    
    /// Check if member of a group
    pub fn is_member_of(&self, gid: GroupId) -> bool {
        self.gid == gid || self.egid == gid || self.groups.contains(&gid)
    }
    
    /// Check if has a specific capability
    pub fn has_capability(&self, cap: super::capability::Capability) -> bool {
        self.capabilities.contains(cap)
    }
    
    /// Set effective user ID (requires CAP_SETUID or appropriate permissions)
    pub fn set_euid(&mut self, new_euid: UserId) -> Result<(), &'static str> {
        // Root can set to any UID
        if self.euid == UserId::ROOT {
            self.euid = new_euid;
            self.fsuid = new_euid;
            return Ok(());
        }
        
        // Non-root can only set to real, effective, or saved UID
        if new_euid == self.uid || new_euid == self.euid || new_euid == self.suid {
            self.euid = new_euid;
            self.fsuid = new_euid;
            Ok(())
        } else {
            Err("Permission denied: cannot set euid")
        }
    }
    
    /// Set real and effective user ID
    pub fn set_reuid(&mut self, ruid: Option<UserId>, euid: Option<UserId>) -> Result<(), &'static str> {
        if self.euid != UserId::ROOT {
            // Non-root has restrictions
            if let Some(new_ruid) = ruid {
                if new_ruid != self.uid && new_ruid != self.euid {
                    return Err("Permission denied: cannot set ruid");
                }
            }
            if let Some(new_euid) = euid {
                if new_euid != self.uid && new_euid != self.euid && new_euid != self.suid {
                    return Err("Permission denied: cannot set euid");
                }
            }
        }
        
        if let Some(new_ruid) = ruid {
            self.uid = new_ruid;
        }
        if let Some(new_euid) = euid {
            // If ruid is set or euid changes, save old euid
            if ruid.is_some() || euid != Some(self.euid) {
                self.suid = self.euid;
            }
            self.euid = new_euid;
            self.fsuid = new_euid;
        }
        
        Ok(())
    }
    
    /// Set all user IDs at once (for setresuid)
    pub fn set_resuid(
        &mut self,
        ruid: Option<UserId>,
        euid: Option<UserId>,
        suid: Option<UserId>,
    ) -> Result<(), &'static str> {
        if self.euid != UserId::ROOT {
            // Non-root can only set to current real, effective, or saved
            let allowed = [self.uid, self.euid, self.suid];
            
            if let Some(new_ruid) = ruid {
                if !allowed.contains(&new_ruid) {
                    return Err("Permission denied: invalid ruid");
                }
            }
            if let Some(new_euid) = euid {
                if !allowed.contains(&new_euid) {
                    return Err("Permission denied: invalid euid");
                }
            }
            if let Some(new_suid) = suid {
                if !allowed.contains(&new_suid) {
                    return Err("Permission denied: invalid suid");
                }
            }
        }
        
        if let Some(new_ruid) = ruid {
            self.uid = new_ruid;
        }
        if let Some(new_euid) = euid {
            self.euid = new_euid;
            self.fsuid = new_euid;
        }
        if let Some(new_suid) = suid {
            self.suid = new_suid;
        }
        
        Ok(())
    }
    
    /// Set effective group ID
    pub fn set_egid(&mut self, new_egid: GroupId) -> Result<(), &'static str> {
        if self.euid == UserId::ROOT {
            self.egid = new_egid;
            self.fsgid = new_egid;
            return Ok(());
        }
        
        if new_egid == self.gid || new_egid == self.egid || new_egid == self.sgid {
            self.egid = new_egid;
            self.fsgid = new_egid;
            Ok(())
        } else {
            Err("Permission denied: cannot set egid")
        }
    }
    
    /// Set supplementary groups
    pub fn set_groups(&mut self, groups: Vec<GroupId>) -> Result<(), &'static str> {
        if self.euid != UserId::ROOT {
            return Err("Permission denied: only root can set groups");
        }
        
        if groups.len() > MAX_GROUPS {
            return Err("Too many groups");
        }
        
        self.groups = groups;
        
        // Update admin status based on wheel membership
        self.is_admin = self.groups.contains(&GroupId::WHEEL);
        
        Ok(())
    }
    
    /// Create credentials for a child process (fork)
    pub fn fork(&self) -> Self {
        self.clone()
    }
    
    /// Transform credentials for exec (handle setuid/setgid)
    pub fn exec(&mut self, file_uid: UserId, file_gid: GroupId, setuid: bool, setgid: bool) {
        if setuid {
            self.euid = file_uid;
            self.suid = file_uid;
            self.fsuid = file_uid;
            
            // If becoming root, grant all capabilities
            if file_uid == UserId::ROOT {
                self.capabilities = CapabilitySet::all();
            }
        }
        
        if setgid {
            self.egid = file_gid;
            self.sgid = file_gid;
            self.fsgid = file_gid;
        }
    }
    
    /// Drop all capabilities (for privilege dropping)
    pub fn drop_capabilities(&mut self) {
        self.capabilities = CapabilitySet::empty();
    }
    
    /// Elevate to root (requires authentication - called by su/sudo)
    pub fn elevate_to_root(&mut self) {
        self.euid = UserId::ROOT;
        self.egid = GroupId::ROOT;
        self.fsuid = UserId::ROOT;
        self.fsgid = GroupId::ROOT;
        self.capabilities = CapabilitySet::all();
    }
}

impl Default for ProcessCredentials {
    fn default() -> Self {
        Self::debos()  // Default to debos user
    }
}

impl core::fmt::Debug for ProcessCredentials {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ProcessCredentials")
            .field("uid", &self.uid)
            .field("euid", &self.euid)
            .field("gid", &self.gid)
            .field("egid", &self.egid)
            .field("is_admin", &self.is_admin)
            .finish()
    }
}

