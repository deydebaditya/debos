//! ICMP Protocol
//!
//! Internet Control Message Protocol for ping and error messages.

use super::Ipv4Address;

/// ICMP types
pub mod IcmpType {
    pub const ECHO_REPLY: u8 = 0;
    pub const DEST_UNREACHABLE: u8 = 3;
    pub const ECHO_REQUEST: u8 = 8;
    pub const TIME_EXCEEDED: u8 = 11;
}

/// ICMP header
#[repr(C, packed)]
pub struct IcmpHeader {
    pub icmp_type: u8,
    pub code: u8,
    pub checksum: [u8; 2],
    pub rest: [u8; 4],
}

/// ICMP Echo header (for ping)
#[repr(C, packed)]
pub struct IcmpEcho {
    pub icmp_type: u8,
    pub code: u8,
    pub checksum: [u8; 2],
    pub identifier: [u8; 2],
    pub sequence: [u8; 2],
}

/// Process ICMP packet
pub fn process_packet(src: Ipv4Address, _dst: Ipv4Address, data: &[u8]) {
    if data.len() < 8 {
        return;
    }
    
    let header = unsafe { &*(data.as_ptr() as *const IcmpHeader) };
    
    match header.icmp_type {
        IcmpType::ECHO_REQUEST => {
            // Would send echo reply here
            crate::println!("ICMP Echo Request from {}", src);
        }
        IcmpType::ECHO_REPLY => {
            let echo = unsafe { &*(data.as_ptr() as *const IcmpEcho) };
            let seq = u16::from_be_bytes(echo.sequence);
            crate::println!("ICMP Echo Reply from {}: seq={}", src, seq);
        }
        IcmpType::DEST_UNREACHABLE => {
            crate::println!("ICMP Destination Unreachable from {}", src);
        }
        IcmpType::TIME_EXCEEDED => {
            crate::println!("ICMP Time Exceeded from {}", src);
        }
        _ => {}
    }
}

/// Create ICMP Echo Request (ping)
pub fn create_echo_request(id: u16, seq: u16, data: &[u8]) -> alloc::vec::Vec<u8> {
    let mut packet = alloc::vec![0u8; 8 + data.len()];
    
    // Type = Echo Request
    packet[0] = IcmpType::ECHO_REQUEST;
    // Code = 0
    packet[1] = 0;
    // Checksum (initially 0)
    packet[2..4].copy_from_slice(&0u16.to_be_bytes());
    // Identifier
    packet[4..6].copy_from_slice(&id.to_be_bytes());
    // Sequence number
    packet[6..8].copy_from_slice(&seq.to_be_bytes());
    // Data
    packet[8..].copy_from_slice(data);
    
    // Calculate checksum
    let cksum = super::ipv4::checksum(&packet);
    packet[2..4].copy_from_slice(&cksum.to_be_bytes());
    
    packet
}

/// Create ICMP Echo Reply
pub fn create_echo_reply(id: u16, seq: u16, data: &[u8]) -> alloc::vec::Vec<u8> {
    let mut packet = alloc::vec![0u8; 8 + data.len()];
    
    packet[0] = IcmpType::ECHO_REPLY;
    packet[1] = 0;
    packet[2..4].copy_from_slice(&0u16.to_be_bytes());
    packet[4..6].copy_from_slice(&id.to_be_bytes());
    packet[6..8].copy_from_slice(&seq.to_be_bytes());
    packet[8..].copy_from_slice(data);
    
    let cksum = super::ipv4::checksum(&packet);
    packet[2..4].copy_from_slice(&cksum.to_be_bytes());
    
    packet
}

