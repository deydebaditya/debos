//! Capability-Based Security
//!
//! The capability system provides fine-grained access control for all
//! kernel resources. Each capability grants specific rights to access
//! a particular resource.
//!
//! ## Design (inspired by seL4):
//! - Capabilities are unforgeable references to kernel objects
//! - Each thread has a Capability Space (CSpace) organized as a tree
//! - Capabilities can be granted, revoked, and delegated

use alloc::collections::BTreeMap;
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};
use bitflags::bitflags;

use crate::scheduler::ThreadId;

/// Capability pointer (index into capability space)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CPtr(pub u64);

/// Capability identifier (globally unique)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapId(pub u64);

/// Next capability ID
static NEXT_CAP_ID: AtomicU64 = AtomicU64::new(1);

bitflags! {
    /// Rights associated with a capability
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CapRights: u32 {
        /// Can read from the object
        const READ = 1 << 0;
        /// Can write to the object
        const WRITE = 1 << 1;
        /// Can execute (for code/endpoints)
        const EXECUTE = 1 << 2;
        /// Can grant capability to others
        const GRANT = 1 << 3;
        /// Can revoke granted capabilities
        const REVOKE = 1 << 4;
        /// Full rights
        const ALL = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits() 
                  | Self::GRANT.bits() | Self::REVOKE.bits();
    }
}

/// Type of kernel object a capability references
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapType {
    /// Null capability (invalid)
    Null,
    /// Memory frame
    Frame,
    /// Page table
    PageTable,
    /// IPC endpoint
    Endpoint,
    /// Thread
    Thread,
    /// IRQ handler
    Irq,
    /// I/O port range
    IoPort,
    /// MMIO region
    Mmio,
}

/// A capability entry
#[derive(Debug, Clone)]
pub struct Capability {
    /// Unique identifier
    pub id: CapId,
    /// Type of object
    pub cap_type: CapType,
    /// Rights mask
    pub rights: CapRights,
    /// Object-specific data (address, ID, etc.)
    pub object: u64,
    /// Parent capability (for revocation tree)
    pub parent: Option<CapId>,
    /// Number of derived capabilities
    pub derived_count: u32,
}

impl Capability {
    /// Create a new capability
    pub fn new(cap_type: CapType, rights: CapRights, object: u64) -> Self {
        Capability {
            id: CapId(NEXT_CAP_ID.fetch_add(1, Ordering::Relaxed)),
            cap_type,
            rights,
            object,
            parent: None,
            derived_count: 0,
        }
    }
    
    /// Check if capability has specific rights
    pub fn has_rights(&self, required: CapRights) -> bool {
        self.rights.contains(required)
    }
    
    /// Derive a new capability with reduced rights
    pub fn derive(&self, new_rights: CapRights) -> Option<Self> {
        // Can only reduce rights
        if !self.rights.contains(new_rights) {
            return None;
        }
        
        // Must have GRANT right to derive
        if !self.has_rights(CapRights::GRANT) {
            return None;
        }
        
        Some(Capability {
            id: CapId(NEXT_CAP_ID.fetch_add(1, Ordering::Relaxed)),
            cap_type: self.cap_type,
            rights: new_rights,
            object: self.object,
            parent: Some(self.id),
            derived_count: 0,
        })
    }
}

/// Capability Space for a thread
#[derive(Debug)]
pub struct CSpace {
    /// Thread that owns this CSpace
    pub owner: ThreadId,
    /// Capability slots (CPtr -> Capability)
    slots: BTreeMap<CPtr, Capability>,
    /// Next available slot
    next_slot: u64,
}

impl CSpace {
    /// Create a new empty capability space
    pub fn new(owner: ThreadId) -> Self {
        CSpace {
            owner,
            slots: BTreeMap::new(),
            next_slot: 1, // Slot 0 is reserved for null
        }
    }
    
    /// Insert a capability and return its pointer
    pub fn insert(&mut self, cap: Capability) -> CPtr {
        let ptr = CPtr(self.next_slot);
        self.next_slot += 1;
        self.slots.insert(ptr, cap);
        ptr
    }
    
    /// Look up a capability by pointer
    pub fn get(&self, ptr: CPtr) -> Option<&Capability> {
        self.slots.get(&ptr)
    }
    
    /// Remove a capability
    pub fn remove(&mut self, ptr: CPtr) -> Option<Capability> {
        self.slots.remove(&ptr)
    }
    
    /// Check if a capability with given rights exists
    pub fn has_cap(&self, ptr: CPtr, required_rights: CapRights) -> bool {
        self.get(ptr)
            .map(|cap| cap.has_rights(required_rights))
            .unwrap_or(false)
    }
}

/// Global capability registry (for revocation tracking)
static CAPABILITIES: Mutex<BTreeMap<CapId, CapType>> = Mutex::new(BTreeMap::new());

/// Per-thread capability spaces
static CSPACES: Mutex<BTreeMap<ThreadId, CSpace>> = Mutex::new(BTreeMap::new());

/// Create a capability space for a thread
pub fn create_cspace(tid: ThreadId) {
    CSPACES.lock().insert(tid, CSpace::new(tid));
}

/// Destroy a thread's capability space
pub fn destroy_cspace(tid: ThreadId) {
    CSPACES.lock().remove(&tid);
}

/// Grant a capability to a thread
pub fn grant(tid: ThreadId, cap: Capability) -> Option<CPtr> {
    let mut cspaces = CSPACES.lock();
    let cspace = cspaces.get_mut(&tid)?;
    
    // Register in global registry
    CAPABILITIES.lock().insert(cap.id, cap.cap_type);
    
    Some(cspace.insert(cap))
}

/// Revoke a capability (and all derived capabilities)
pub fn revoke(tid: ThreadId, ptr: CPtr) -> bool {
    let mut cspaces = CSPACES.lock();
    
    if let Some(cspace) = cspaces.get_mut(&tid) {
        if let Some(cap) = cspace.remove(ptr) {
            // TODO: Recursively revoke all derived capabilities
            CAPABILITIES.lock().remove(&cap.id);
            return true;
        }
    }
    
    false
}

/// Validate a capability reference
pub fn validate(tid: ThreadId, ptr: CPtr, cap_type: CapType, required_rights: CapRights) -> bool {
    let cspaces = CSPACES.lock();
    
    if let Some(cspace) = cspaces.get(&tid) {
        if let Some(cap) = cspace.get(ptr) {
            return cap.cap_type == cap_type && cap.has_rights(required_rights);
        }
    }
    
    false
}

