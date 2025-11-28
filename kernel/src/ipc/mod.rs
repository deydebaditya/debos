//! Inter-Process Communication (IPC)
//!
//! The IPC subsystem is the heart of the DebOS microkernel. It replaces
//! function calls that would exist in a monolithic kernel, enabling
//! communication between userspace servers.
//!
//! ## Design Philosophy
//! - Synchronous RPC-style communication
//! - Zero-copy for large messages (via shared memory)
//! - Direct switch optimization (L4-style)

pub mod endpoint;
pub mod message;

use alloc::collections::BTreeMap;
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};

use endpoint::Endpoint;
pub use crate::scheduler::ThreadId;
use crate::scheduler::{block_current, unblock};

/// Endpoint ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EndpointId(pub u64);

/// Next endpoint ID
static NEXT_ENDPOINT_ID: AtomicU64 = AtomicU64::new(1);

/// Global endpoint registry
static ENDPOINTS: Mutex<BTreeMap<EndpointId, Endpoint>> = Mutex::new(BTreeMap::new());

/// Threads waiting for IPC replies
static WAITING_FOR_REPLY: Mutex<BTreeMap<ThreadId, EndpointId>> = Mutex::new(BTreeMap::new());

/// Create a new IPC endpoint
pub fn create_endpoint() -> EndpointId {
    let id = EndpointId(NEXT_ENDPOINT_ID.fetch_add(1, Ordering::Relaxed));
    
    ENDPOINTS.lock().insert(id, Endpoint::new(id));
    
    id
}

/// Destroy an endpoint
pub fn destroy_endpoint(id: EndpointId) {
    ENDPOINTS.lock().remove(&id);
}

/// Send a message and wait for reply (blocking call)
/// 
/// This is the primary IPC mechanism - synchronous RPC-style communication.
pub fn ipc_call(
    endpoint: EndpointId,
    message: &[u8],
    reply_buffer: &mut [u8],
) -> Result<usize, IpcError> {
    let current_tid = crate::scheduler::current_tid()
        .ok_or(IpcError::InvalidThread)?;
    
    // Check if endpoint exists and has a waiting receiver
    let receiver_tid = {
        let mut endpoints = ENDPOINTS.lock();
        let ep = endpoints.get_mut(&endpoint).ok_or(IpcError::InvalidEndpoint)?;
        
        // Copy message to endpoint buffer
        ep.set_message(message, current_tid)?;
        
        // Get receiver if one is waiting
        ep.take_waiting_receiver()
    };
    
    // Mark ourselves as waiting for reply
    WAITING_FOR_REPLY.lock().insert(current_tid, endpoint);
    
    // If there's a receiver, wake them up (direct switch optimization candidate)
    if let Some(receiver) = receiver_tid {
        unblock(receiver);
    }
    
    // Block until we get a reply
    block_current();
    
    // We've been woken up - get the reply
    let reply_len = {
        let mut endpoints = ENDPOINTS.lock();
        let ep = endpoints.get_mut(&endpoint).ok_or(IpcError::InvalidEndpoint)?;
        ep.get_reply(reply_buffer)?
    };
    
    WAITING_FOR_REPLY.lock().remove(&current_tid);
    
    Ok(reply_len)
}

/// Wait for a message on an endpoint (server-side)
pub fn ipc_wait(endpoint: EndpointId, buffer: &mut [u8]) -> Result<(usize, ThreadId), IpcError> {
    let current_tid = crate::scheduler::current_tid()
        .ok_or(IpcError::InvalidThread)?;
    
    loop {
        // Check if there's a message waiting
        let result = {
            let mut endpoints = ENDPOINTS.lock();
            let ep = endpoints.get_mut(&endpoint).ok_or(IpcError::InvalidEndpoint)?;
            
            if let Some((msg, sender)) = ep.take_message() {
                let len = core::cmp::min(msg.len(), buffer.len());
                buffer[..len].copy_from_slice(&msg[..len]);
                Some((len, sender))
            } else {
                // No message, register as waiting receiver
                ep.set_waiting_receiver(current_tid);
                None
            }
        };
        
        if let Some(result) = result {
            return Ok(result);
        }
        
        // Block until a message arrives
        block_current();
    }
}

/// Send a reply to a waiting caller
pub fn ipc_reply(endpoint: EndpointId, caller: ThreadId, reply: &[u8]) -> Result<(), IpcError> {
    // Store reply in endpoint
    {
        let mut endpoints = ENDPOINTS.lock();
        let ep = endpoints.get_mut(&endpoint).ok_or(IpcError::InvalidEndpoint)?;
        ep.set_reply(reply)?;
    }
    
    // Wake up the caller
    unblock(caller);
    
    Ok(())
}

/// IPC error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcError {
    /// Invalid endpoint ID
    InvalidEndpoint,
    /// Invalid thread
    InvalidThread,
    /// Message too large
    MessageTooLarge,
    /// Buffer too small
    BufferTooSmall,
    /// Endpoint closed
    EndpointClosed,
    /// Operation timed out
    Timeout,
}
