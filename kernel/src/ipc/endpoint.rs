//! IPC Endpoint
//!
//! An endpoint is a communication channel that threads can send messages to
//! and receive messages from.

use alloc::vec::Vec;
use super::{EndpointId, IpcError, ThreadId};

/// Maximum message size (4KB)
pub const MAX_MESSAGE_SIZE: usize = 4096;

/// IPC Endpoint
pub struct Endpoint {
    /// Endpoint ID
    pub id: EndpointId,
    /// Thread waiting to receive on this endpoint
    waiting_receiver: Option<ThreadId>,
    /// Current message buffer
    message_buffer: Vec<u8>,
    /// Sender of the current message
    message_sender: Option<ThreadId>,
    /// Reply buffer
    reply_buffer: Vec<u8>,
    /// Whether the endpoint is closed
    closed: bool,
}

impl Endpoint {
    /// Create a new endpoint
    pub fn new(id: EndpointId) -> Self {
        Endpoint {
            id,
            waiting_receiver: None,
            message_buffer: Vec::new(),
            message_sender: None,
            reply_buffer: Vec::new(),
            closed: false,
        }
    }
    
    /// Set the waiting receiver
    pub fn set_waiting_receiver(&mut self, tid: ThreadId) {
        self.waiting_receiver = Some(tid);
    }
    
    /// Take the waiting receiver (if any)
    pub fn take_waiting_receiver(&mut self) -> Option<ThreadId> {
        self.waiting_receiver.take()
    }
    
    /// Set a message to be delivered
    pub fn set_message(&mut self, data: &[u8], sender: ThreadId) -> Result<(), IpcError> {
        if self.closed {
            return Err(IpcError::EndpointClosed);
        }
        
        if data.len() > MAX_MESSAGE_SIZE {
            return Err(IpcError::MessageTooLarge);
        }
        
        self.message_buffer = data.to_vec();
        self.message_sender = Some(sender);
        
        Ok(())
    }
    
    /// Take the pending message (if any)
    pub fn take_message(&mut self) -> Option<(Vec<u8>, ThreadId)> {
        if self.message_buffer.is_empty() {
            return None;
        }
        
        let msg = core::mem::take(&mut self.message_buffer);
        let sender = self.message_sender.take()?;
        
        Some((msg, sender))
    }
    
    /// Set a reply to be delivered
    pub fn set_reply(&mut self, data: &[u8]) -> Result<(), IpcError> {
        if data.len() > MAX_MESSAGE_SIZE {
            return Err(IpcError::MessageTooLarge);
        }
        
        self.reply_buffer = data.to_vec();
        
        Ok(())
    }
    
    /// Get the reply
    pub fn get_reply(&mut self, buffer: &mut [u8]) -> Result<usize, IpcError> {
        if self.reply_buffer.is_empty() {
            return Ok(0);
        }
        
        let len = core::cmp::min(self.reply_buffer.len(), buffer.len());
        buffer[..len].copy_from_slice(&self.reply_buffer[..len]);
        
        self.reply_buffer.clear();
        
        Ok(len)
    }
    
    /// Close the endpoint
    pub fn close(&mut self) {
        self.closed = true;
    }
    
    /// Check if the endpoint is closed
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

