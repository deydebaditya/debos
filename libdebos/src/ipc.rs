//! IPC API
//!
//! Provides Inter-Process Communication (IPC) abstractions for DebOS.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use libdebos::ipc::Endpoint;
//!
//! // Connect to VFS server endpoint
//! let vfs = Endpoint::from_id(1000);
//!
//! // Make an IPC call
//! let mut reply = [0u8; 4096];
//! let bytes = vfs.call(&request, &mut reply)?;
//! ```

use crate::syscall::{self, *};

/// Well-known endpoint IDs
pub mod well_known {
    /// VFS Server endpoint
    pub const VFS_SERVER: u64 = 1000;
    /// Network Server endpoint
    pub const NET_SERVER: u64 = 1001;
    /// Device Manager endpoint
    pub const DEV_MANAGER: u64 = 1002;
    /// Window Server endpoint
    pub const WINDOW_SERVER: u64 = 1003;
}

/// IPC Endpoint handle
pub struct Endpoint {
    id: u64,
}

impl Endpoint {
    /// Create an endpoint handle from a well-known ID
    pub const fn from_id(id: u64) -> Self {
        Self { id }
    }
    
    /// Get the endpoint ID
    pub fn id(&self) -> u64 {
        self.id
    }
}

impl Endpoint {
    /// Make an IPC call (send + receive reply)
    pub fn call(&self, message: &[u8], reply: &mut [u8]) -> Result<usize, i64> {
        let result = unsafe {
            syscall::syscall(
                SYS_IPC_CALL,
                self.id,
                message.as_ptr() as u64,
                message.len() as u64,
                reply.as_mut_ptr() as u64,
                reply.len() as u64,
                0,
            )
        };
        
        if result < 0 {
            Err(result)
        } else {
            Ok(result as usize)
        }
    }
    
    /// Wait for a message on this endpoint
    pub fn wait(&self, buffer: &mut [u8]) -> Result<usize, i64> {
        let result = unsafe {
            syscall::syscall(
                SYS_IPC_WAIT,
                self.id,
                buffer.as_mut_ptr() as u64,
                buffer.len() as u64,
                0,
                0,
                0,
            )
        };
        
        if result < 0 {
            Err(result)
        } else {
            Ok(result as usize)
        }
    }
}

