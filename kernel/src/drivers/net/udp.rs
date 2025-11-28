//! UDP Protocol
//!
//! User Datagram Protocol for connectionless communication.

use super::Ipv4Address;

/// UDP header size
pub const UDP_HEADER_SIZE: usize = 8;

/// UDP header
#[repr(C, packed)]
pub struct UdpHeader {
    pub src_port: [u8; 2],
    pub dst_port: [u8; 2],
    pub length: [u8; 2],
    pub checksum: [u8; 2],
}

impl UdpHeader {
    pub fn source_port(&self) -> u16 {
        u16::from_be_bytes(self.src_port)
    }
    
    pub fn destination_port(&self) -> u16 {
        u16::from_be_bytes(self.dst_port)
    }
    
    pub fn length(&self) -> u16 {
        u16::from_be_bytes(self.length)
    }
}

/// Process UDP packet
pub fn process_packet(src_ip: Ipv4Address, _dst_ip: Ipv4Address, data: &[u8]) {
    if data.len() < UDP_HEADER_SIZE {
        return;
    }
    
    let header = unsafe { &*(data.as_ptr() as *const UdpHeader) };
    let _payload = &data[UDP_HEADER_SIZE..];
    
    // Find socket listening on this port and deliver data
    let dst_port = header.destination_port();
    let src_port = header.source_port();
    
    // Would deliver to socket here
    crate::println!("UDP: {}:{} -> port {}", src_ip, src_port, dst_port);
}

/// Create UDP packet
pub fn create_packet(
    src_ip: Ipv4Address,
    dst_ip: Ipv4Address,
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> alloc::vec::Vec<u8> {
    let length = (UDP_HEADER_SIZE + payload.len()) as u16;
    
    let mut packet = alloc::vec![0u8; UDP_HEADER_SIZE + payload.len()];
    
    // Source port
    packet[0..2].copy_from_slice(&src_port.to_be_bytes());
    // Destination port
    packet[2..4].copy_from_slice(&dst_port.to_be_bytes());
    // Length
    packet[4..6].copy_from_slice(&length.to_be_bytes());
    // Checksum (0 = disabled for UDP)
    packet[6..8].copy_from_slice(&0u16.to_be_bytes());
    // Payload
    packet[UDP_HEADER_SIZE..].copy_from_slice(payload);
    
    // Calculate checksum (optional for UDP but recommended)
    let cksum = udp_checksum(src_ip, dst_ip, &packet);
    packet[6..8].copy_from_slice(&cksum.to_be_bytes());
    
    packet
}

/// Calculate UDP checksum with pseudo-header
fn udp_checksum(src_ip: Ipv4Address, dst_ip: Ipv4Address, packet: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    
    // Pseudo-header
    sum += u16::from_be_bytes([src_ip.0[0], src_ip.0[1]]) as u32;
    sum += u16::from_be_bytes([src_ip.0[2], src_ip.0[3]]) as u32;
    sum += u16::from_be_bytes([dst_ip.0[0], dst_ip.0[1]]) as u32;
    sum += u16::from_be_bytes([dst_ip.0[2], dst_ip.0[3]]) as u32;
    sum += 17u32;  // Protocol = UDP
    sum += packet.len() as u32;  // UDP length
    
    // UDP packet
    let mut i = 0;
    while i + 1 < packet.len() {
        sum += u16::from_be_bytes([packet[i], packet[i + 1]]) as u32;
        i += 2;
    }
    if i < packet.len() {
        sum += (packet[i] as u32) << 8;
    }
    
    // Fold to 16 bits
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    
    let cksum = !sum as u16;
    if cksum == 0 { 0xFFFF } else { cksum }
}

