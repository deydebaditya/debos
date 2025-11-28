//! Capability System
//!
//! Fine-grained privileges that replace all-or-nothing root access.
//! Based on POSIX capabilities with DebOS extensions.

use core::fmt;

/// System capabilities
///
/// Each capability grants a specific privilege that would traditionally
/// require root access.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Capability {
    // === File Capabilities ===
    
    /// Override file read/write/execute permission checks
    DacOverride = 0,
    
    /// Override file read permission checks and directory read/execute
    DacReadSearch = 1,
    
    /// Bypass permission checks on file operations that require ownership
    Fowner = 2,
    
    /// Don't clear set-user-ID and set-group-ID bits when file is modified
    Fsetid = 3,
    
    // === Process Capabilities ===
    
    /// Bypass permission checks for sending signals
    Kill = 4,
    
    /// Make arbitrary manipulations of process UIDs
    Setuid = 5,
    
    /// Make arbitrary manipulations of process GIDs
    Setgid = 6,
    
    /// Modify capability sets of other processes
    Setpcap = 7,
    
    // === Network Capabilities ===
    
    /// Bind a socket to privileged ports (< 1024)
    NetBindService = 8,
    
    /// Use RAW and PACKET sockets
    NetRaw = 9,
    
    /// Perform network administration tasks
    NetAdmin = 10,
    
    // === System Capabilities ===
    
    /// Change file ownership
    Chown = 11,
    
    /// Lock memory (mlock, mlockall)
    IpcLock = 12,
    
    /// Load and unload kernel modules
    SysModule = 13,
    
    /// Perform I/O port operations
    SysRawio = 14,
    
    /// Use chroot()
    SysChroot = 15,
    
    /// Trace any process with ptrace
    SysPtrace = 16,
    
    /// Set system time
    SysTime = 17,
    
    /// Configure audit subsystem
    Audit = 18,
    
    /// Perform system administration tasks
    SysAdmin = 19,
    
    /// Reboot and control system
    SysBoot = 20,
    
    /// Raise process nice value, set real-time scheduling
    SysNice = 21,
    
    /// Override resource limits
    SysResource = 22,
    
    /// Use syslog()
    Syslog = 23,
    
    // === DebOS Extensions ===
    
    /// Access kernel debugging facilities
    DebosDebug = 32,
    
    /// Modify security policies
    DebosPolicy = 33,
    
    /// Manage system services
    DebosService = 34,
    
    /// Create/manage users and groups
    DebosUserAdmin = 35,
}

impl Capability {
    /// Get the bit position for this capability
    pub fn bit(&self) -> u64 {
        1u64 << (*self as u8)
    }
    
    /// Get capability name as string
    pub fn name(&self) -> &'static str {
        match self {
            Capability::DacOverride => "CAP_DAC_OVERRIDE",
            Capability::DacReadSearch => "CAP_DAC_READ_SEARCH",
            Capability::Fowner => "CAP_FOWNER",
            Capability::Fsetid => "CAP_FSETID",
            Capability::Kill => "CAP_KILL",
            Capability::Setuid => "CAP_SETUID",
            Capability::Setgid => "CAP_SETGID",
            Capability::Setpcap => "CAP_SETPCAP",
            Capability::NetBindService => "CAP_NET_BIND_SERVICE",
            Capability::NetRaw => "CAP_NET_RAW",
            Capability::NetAdmin => "CAP_NET_ADMIN",
            Capability::Chown => "CAP_CHOWN",
            Capability::IpcLock => "CAP_IPC_LOCK",
            Capability::SysModule => "CAP_SYS_MODULE",
            Capability::SysRawio => "CAP_SYS_RAWIO",
            Capability::SysChroot => "CAP_SYS_CHROOT",
            Capability::SysPtrace => "CAP_SYS_PTRACE",
            Capability::SysTime => "CAP_SYS_TIME",
            Capability::Audit => "CAP_AUDIT",
            Capability::SysAdmin => "CAP_SYS_ADMIN",
            Capability::SysBoot => "CAP_SYS_BOOT",
            Capability::SysNice => "CAP_SYS_NICE",
            Capability::SysResource => "CAP_SYS_RESOURCE",
            Capability::Syslog => "CAP_SYSLOG",
            Capability::DebosDebug => "CAP_DEBOS_DEBUG",
            Capability::DebosPolicy => "CAP_DEBOS_POLICY",
            Capability::DebosService => "CAP_DEBOS_SERVICE",
            Capability::DebosUserAdmin => "CAP_DEBOS_USER_ADMIN",
        }
    }
}

/// Set of capabilities (bitmap)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CapabilitySet(u64);

impl CapabilitySet {
    /// Empty capability set
    pub const fn empty() -> Self {
        CapabilitySet(0)
    }
    
    /// All capabilities (root)
    pub const fn all() -> Self {
        CapabilitySet(!0u64)
    }
    
    /// Default capabilities for regular users
    pub fn user_default() -> Self {
        CapabilitySet(0)
    }
    
    /// Default capabilities for administrators
    pub fn admin_default() -> Self {
        let mut caps = CapabilitySet::empty();
        // Admins can manage users and services, but not everything
        caps.add(Capability::DebosUserAdmin);
        caps.add(Capability::DebosService);
        caps.add(Capability::SysNice);
        caps.add(Capability::Kill);
        caps
    }
    
    /// Capabilities for network services
    pub fn network_service() -> Self {
        let mut caps = CapabilitySet::empty();
        caps.add(Capability::NetBindService);
        caps.add(Capability::NetRaw);
        caps
    }
    
    /// Check if capability is present
    pub fn contains(&self, cap: Capability) -> bool {
        (self.0 & cap.bit()) != 0
    }
    
    /// Add a capability
    pub fn add(&mut self, cap: Capability) {
        self.0 |= cap.bit();
    }
    
    /// Remove a capability
    pub fn remove(&mut self, cap: Capability) {
        self.0 &= !cap.bit();
    }
    
    /// Union of two capability sets
    pub fn union(&self, other: &CapabilitySet) -> CapabilitySet {
        CapabilitySet(self.0 | other.0)
    }
    
    /// Intersection of two capability sets
    pub fn intersection(&self, other: &CapabilitySet) -> CapabilitySet {
        CapabilitySet(self.0 & other.0)
    }
    
    /// Check if this set is empty
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
    
    /// Check if this set has all capabilities
    pub fn is_full(&self) -> bool {
        self.0 == !0u64
    }
    
    /// Get raw value
    pub fn as_raw(&self) -> u64 {
        self.0
    }
    
    /// Create from raw value
    pub fn from_raw(raw: u64) -> Self {
        CapabilitySet(raw)
    }
}

impl fmt::Debug for CapabilitySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_full() {
            write!(f, "CapabilitySet(ALL)")
        } else if self.is_empty() {
            write!(f, "CapabilitySet(EMPTY)")
        } else {
            write!(f, "CapabilitySet({:#x})", self.0)
        }
    }
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self::user_default()
    }
}

