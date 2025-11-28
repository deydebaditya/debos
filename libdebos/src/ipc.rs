//! IPC API

use crate::syscall::{self, *};

/// IPC Endpoint handle
pub struct Endpoint {
    id: u64,
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

