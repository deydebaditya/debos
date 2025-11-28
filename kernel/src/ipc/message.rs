//! IPC Message Types
//!
//! Defines structured message formats for IPC communication.

/// IPC Message Header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MessageHeader {
    /// Message type/operation code
    pub msg_type: u32,
    /// Flags
    pub flags: u32,
    /// Payload length
    pub length: u32,
    /// Reserved for future use
    pub reserved: u32,
}

impl MessageHeader {
    /// Create a new message header
    pub const fn new(msg_type: u32, length: u32) -> Self {
        MessageHeader {
            msg_type,
            flags: 0,
            length,
            reserved: 0,
        }
    }
    
    /// Size of the header in bytes
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }
}

/// Message flags
pub mod flags {
    /// Message requires a reply
    pub const NEED_REPLY: u32 = 1 << 0;
    /// Message is a reply
    pub const IS_REPLY: u32 = 1 << 1;
    /// Message contains capability references
    pub const HAS_CAPS: u32 = 1 << 2;
    /// Message uses shared memory
    pub const SHARED_MEM: u32 = 1 << 3;
}

/// Standard message types for system services
pub mod msg_types {
    // VFS operations
    pub const VFS_OPEN: u32 = 0x0100;
    pub const VFS_READ: u32 = 0x0101;
    pub const VFS_WRITE: u32 = 0x0102;
    pub const VFS_CLOSE: u32 = 0x0103;
    pub const VFS_STAT: u32 = 0x0104;
    pub const VFS_MKDIR: u32 = 0x0105;
    pub const VFS_UNLINK: u32 = 0x0106;
    
    // Network operations
    pub const NET_SOCKET: u32 = 0x0200;
    pub const NET_BIND: u32 = 0x0201;
    pub const NET_LISTEN: u32 = 0x0202;
    pub const NET_ACCEPT: u32 = 0x0203;
    pub const NET_CONNECT: u32 = 0x0204;
    pub const NET_SEND: u32 = 0x0205;
    pub const NET_RECV: u32 = 0x0206;
    pub const NET_CLOSE: u32 = 0x0207;
    
    // Device operations
    pub const DEV_ENUMERATE: u32 = 0x0300;
    pub const DEV_OPEN: u32 = 0x0301;
    pub const DEV_IOCTL: u32 = 0x0302;
    
    // System operations
    pub const SYS_PING: u32 = 0x0001;
    pub const SYS_SHUTDOWN: u32 = 0x0002;
}

/// A structured IPC message
#[derive(Debug)]
pub struct Message {
    /// Message header
    pub header: MessageHeader,
    /// Message payload
    pub payload: [u8; 4080], // 4096 - 16 (header size)
}

impl Message {
    /// Create a new empty message
    pub const fn new() -> Self {
        Message {
            header: MessageHeader::new(0, 0),
            payload: [0; 4080],
        }
    }
    
    /// Create a message with type and payload
    pub fn with_payload(msg_type: u32, data: &[u8]) -> Self {
        let mut msg = Self::new();
        msg.header.msg_type = msg_type;
        msg.header.length = data.len() as u32;
        
        let len = core::cmp::min(data.len(), msg.payload.len());
        msg.payload[..len].copy_from_slice(&data[..len]);
        
        msg
    }
    
    /// Get payload as slice
    pub fn payload_slice(&self) -> &[u8] {
        &self.payload[..self.header.length as usize]
    }
    
    /// Set payload data
    pub fn set_payload(&mut self, data: &[u8]) {
        let len = core::cmp::min(data.len(), self.payload.len());
        self.payload[..len].copy_from_slice(&data[..len]);
        self.header.length = len as u32;
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

