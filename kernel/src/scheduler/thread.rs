//! Thread Management
//!
//! Defines the Thread Control Block (TCB) and thread-related types.

use alloc::boxed::Box;
use core::fmt;

use crate::arch::ArchContext;
use crate::security::credentials::ProcessCredentials;

/// Unique identifier for a thread
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThreadId(pub u64);

impl fmt::Display for ThreadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Thread state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// Thread is ready to run
    Ready,
    /// Thread is currently running on a CPU
    Running,
    /// Thread is waiting for an event
    Blocked,
    /// Thread has exited
    Terminated,
}

/// Priority class for threads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityClass {
    /// Real-time priority (0-31): Hard priority, no aging
    RealTime,
    /// Normal priority (32-255): Dynamic priority with aging
    Normal,
}

impl PriorityClass {
    /// Determine priority class from priority value
    pub fn from_priority(priority: u8) -> Self {
        if priority < 32 {
            PriorityClass::RealTime
        } else {
            PriorityClass::Normal
        }
    }
}

/// Thread Control Block (TCB)
/// 
/// Contains all information needed to manage a thread.
pub struct Thread {
    /// Unique thread identifier
    pub id: ThreadId,
    /// Current thread state
    pub state: ThreadState,
    /// Thread priority (0-255, lower = higher priority)
    pub priority: u8,
    /// Current dynamic priority (may differ from base priority due to aging)
    pub dynamic_priority: u8,
    /// CPU context (registers, etc.)
    pub context: ArchContext,
    /// Kernel stack
    pub kernel_stack: Box<[u8]>,
    /// Remaining time slice (in ticks)
    pub time_slice: u32,
    /// CPU affinity (None = any CPU)
    pub cpu_affinity: Option<u8>,
    /// Total CPU time consumed (in ticks)
    pub cpu_time: u64,
    /// Thread name (for debugging)
    pub name: [u8; 32],
    /// Process credentials (user, group, capabilities)
    pub credentials: ProcessCredentials,
}

impl Thread {
    /// Create a new thread with default (debos) credentials
    pub fn new(
        id: ThreadId,
        priority: u8,
        context: ArchContext,
        kernel_stack: Box<[u8]>,
    ) -> Self {
        Self::with_credentials(id, priority, context, kernel_stack, ProcessCredentials::default())
    }
    
    /// Create a new thread with specific credentials
    pub fn with_credentials(
        id: ThreadId,
        priority: u8,
        context: ArchContext,
        kernel_stack: Box<[u8]>,
        credentials: ProcessCredentials,
    ) -> Self {
        Thread {
            id,
            state: ThreadState::Ready,
            priority,
            dynamic_priority: priority,
            context,
            kernel_stack,
            time_slice: Self::time_slice_for_priority(priority),
            cpu_affinity: None,
            cpu_time: 0,
            name: [0; 32],
            credentials,
        }
    }
    
    /// Create a kernel thread (runs as root)
    pub fn kernel(
        id: ThreadId,
        priority: u8,
        context: ArchContext,
        kernel_stack: Box<[u8]>,
    ) -> Self {
        Self::with_credentials(id, priority, context, kernel_stack, ProcessCredentials::kernel())
    }
    
    /// Calculate time slice based on priority
    fn time_slice_for_priority(priority: u8) -> u32 {
        // Higher priority (lower number) = longer time slice
        // Real-time: 50-100 ticks
        // Normal: 10-50 ticks
        if priority < 32 {
            100 - (priority as u32 * 2)
        } else {
            50 - ((priority - 32) as u32 / 5)
        }
    }
    
    /// Get default time slice for this thread
    pub fn default_time_slice(&self) -> u32 {
        Self::time_slice_for_priority(self.priority)
    }
    
    /// Get priority class
    pub fn priority_class(&self) -> PriorityClass {
        PriorityClass::from_priority(self.priority)
    }
    
    /// Set thread name
    pub fn set_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = core::cmp::min(bytes.len(), 31);
        self.name[..len].copy_from_slice(&bytes[..len]);
        self.name[len] = 0;
    }
    
    /// Get thread name as string
    pub fn name_str(&self) -> &str {
        let len = self.name.iter().position(|&c| c == 0).unwrap_or(32);
        core::str::from_utf8(&self.name[..len]).unwrap_or("<invalid>")
    }
}

impl fmt::Debug for Thread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Thread")
            .field("id", &self.id)
            .field("state", &self.state)
            .field("priority", &self.priority)
            .field("name", &self.name_str())
            .finish()
    }
}

