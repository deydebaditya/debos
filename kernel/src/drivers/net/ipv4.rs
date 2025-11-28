//! IPv4 Protocol
//!
//! Internet Protocol version 4 packet handling.

use super::{Ipv4Address, icmp, udp, tcp};

/// IPv4 header minimum size
pub const IPV4_HEADER_MIN: usize = 20;

/// IP protocol numbers
pub mod Protocol {
    pub const ICMP: u8 = 1;
    pub const TCP: u8 = 6;
    pub const UDP: u8 = 17;
}

/// IPv4 header
#[repr(C, packed)]
pub struct Ipv4Header {
    /// Version (4 bits) + IHL (4 bits)
    pub version_ihl: u8,
    /// DSCP (6 bits) + ECN (2 bits)
    pub dscp_ecn: u8,
    /// Total length
    pub total_length: [u8; 2],
    /// Identification
    pub identification: [u8; 2],
    /// Flags (3 bits) + Fragment offset (13 bits)
    pub flags_fragment: [u8; 2],
    /// Time to live
    pub ttl: u8,
    /// Protocol
    pub protocol: u8,
    /// Header checksum
    pub checksum: [u8; 2],
    /// Source address
    pub src_addr: [u8; 4],
    /// Destination address
    pub dst_addr: [u8; 4],
}

impl Ipv4Header {
    /// Get IP version
    pub fn version(&self) -> u8 {
        (self.version_ihl >> 4) & 0x0F
    }
    
    /// Get header length in bytes
    pub fn header_length(&self) -> usize {
        ((self.version_ihl & 0x0F) as usize) * 4
    }
    
    /// Get total packet length
    pub fn total_length(&self) -> u16 {
        u16::from_be_bytes(self.total_length)
    }
    
    /// Get source address
    pub fn source(&self) -> Ipv4Address {
        Ipv4Address(self.src_addr)
    }
    
    /// Get destination address
    pub fn destination(&self) -> Ipv4Address {
        Ipv4Address(self.dst_addr)
    }
    
    /// Calculate header checksum
    pub fn calculate_checksum(&self) -> u16 {
        let header_len = self.header_length();
        let bytes = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, header_len)
        };
        
        checksum(bytes)
    }
    
    /// Verify header checksum
    pub fn verify_checksum(&self) -> bool {
        self.calculate_checksum() == 0
    }
}

/// Calculate IP checksum
pub fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    
    while i + 1 < data.len() {
        sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }
    
    // Handle odd byte
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }
    
    // Fold 32-bit sum to 16 bits
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    
    !sum as u16
}

/// Process IPv4 packet
pub fn process_packet(data: &[u8]) {
    if data.len() < IPV4_HEADER_MIN {
        return;
    }
    
    let header = unsafe { &*(data.as_ptr() as *const Ipv4Header) };
    
    // Check version
    if header.version() != 4 {
        return;
    }
    
    let header_len = header.header_length();
    if data.len() < header_len {
        return;
    }
    
    // Get payload
    let payload = &data[header_len..];
    
    match header.protocol {
        Protocol::ICMP => {
            icmp::process_packet(header.source(), header.destination(), payload);
        }
        Protocol::UDP => {
            udp::process_packet(header.source(), header.destination(), payload);
        }
        Protocol::TCP => {
            tcp::process_packet(header.source(), header.destination(), payload);
        }
        _ => {
            // Unknown protocol
        }
    }
}

/// Create IPv4 packet
pub fn create_packet(
    src: Ipv4Address,
    dst: Ipv4Address,
    protocol: u8,
    ttl: u8,
    payload: &[u8],
) -> alloc::vec::Vec<u8> {
    let total_len = (IPV4_HEADER_MIN + payload.len()) as u16;
    
    let mut packet = alloc::vec![0u8; IPV4_HEADER_MIN + payload.len()];
    
    // Version (4) + IHL (5 = 20 bytes)
    packet[0] = 0x45;
    // DSCP + ECN
    packet[1] = 0;
    // Total length
    packet[2..4].copy_from_slice(&total_len.to_be_bytes());
    // Identification
    packet[4..6].copy_from_slice(&0u16.to_be_bytes());
    // Flags + Fragment offset
    packet[6..8].copy_from_slice(&0u16.to_be_bytes());
    // TTL
    packet[8] = ttl;
    // Protocol
    packet[9] = protocol;
    // Checksum (initially 0)
    packet[10..12].copy_from_slice(&0u16.to_be_bytes());
    // Source address
    packet[12..16].copy_from_slice(&src.0);
    // Destination address
    packet[16..20].copy_from_slice(&dst.0);
    
    // Copy payload
    packet[IPV4_HEADER_MIN..].copy_from_slice(payload);
    
    // Calculate and set checksum
    let cksum = checksum(&packet[0..IPV4_HEADER_MIN]);
    packet[10..12].copy_from_slice(&cksum.to_be_bytes());
    
    packet
}

