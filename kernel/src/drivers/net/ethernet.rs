//! Ethernet Frame Handling
//!
//! Parses and creates Ethernet frames.

use super::{MacAddress, arp, ipv4};

/// Ethernet header size
pub const ETH_HEADER_SIZE: usize = 14;

/// EtherType values
pub mod EtherType {
    pub const IPV4: u16 = 0x0800;
    pub const ARP: u16 = 0x0806;
    pub const IPV6: u16 = 0x86DD;
}

/// Ethernet frame header
#[repr(C, packed)]
pub struct EthernetHeader {
    pub dst_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ether_type: [u8; 2],
}

impl EthernetHeader {
    /// Get destination MAC
    pub fn destination(&self) -> MacAddress {
        MacAddress(self.dst_mac)
    }
    
    /// Get source MAC
    pub fn source(&self) -> MacAddress {
        MacAddress(self.src_mac)
    }
    
    /// Get EtherType
    pub fn ether_type(&self) -> u16 {
        u16::from_be_bytes(self.ether_type)
    }
}

/// Parse an Ethernet frame
pub fn parse_frame(data: &[u8]) -> Option<(&EthernetHeader, &[u8])> {
    if data.len() < ETH_HEADER_SIZE {
        return None;
    }
    
    let header = unsafe { &*(data.as_ptr() as *const EthernetHeader) };
    let payload = &data[ETH_HEADER_SIZE..];
    
    Some((header, payload))
}

/// Process a received Ethernet frame
pub fn process_frame(data: &[u8]) {
    if let Some((header, payload)) = parse_frame(data) {
        match header.ether_type() {
            EtherType::ARP => {
                arp::process_packet(payload);
            }
            EtherType::IPV4 => {
                ipv4::process_packet(payload);
            }
            EtherType::IPV6 => {
                // IPv6 not implemented yet
            }
            _ => {
                // Unknown EtherType
            }
        }
    }
}

/// Create an Ethernet frame
pub fn create_frame(dst: MacAddress, src: MacAddress, ether_type: u16, payload: &[u8]) -> alloc::vec::Vec<u8> {
    let mut frame = alloc::vec::Vec::with_capacity(ETH_HEADER_SIZE + payload.len());
    
    frame.extend_from_slice(&dst.0);
    frame.extend_from_slice(&src.0);
    frame.extend_from_slice(&ether_type.to_be_bytes());
    frame.extend_from_slice(payload);
    
    frame
}

