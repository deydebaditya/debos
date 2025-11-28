//! TCP Protocol
//!
//! Transmission Control Protocol for reliable, ordered communication.

use alloc::collections::{BTreeMap, VecDeque};
use spin::Mutex;
use lazy_static::lazy_static;

use super::Ipv4Address;

/// TCP header minimum size
pub const TCP_HEADER_MIN: usize = 20;

/// TCP flags
pub mod TcpFlags {
    pub const FIN: u8 = 0x01;
    pub const SYN: u8 = 0x02;
    pub const RST: u8 = 0x04;
    pub const PSH: u8 = 0x08;
    pub const ACK: u8 = 0x10;
    pub const URG: u8 = 0x20;
}

/// TCP connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

/// TCP header
#[repr(C, packed)]
pub struct TcpHeader {
    pub src_port: [u8; 2],
    pub dst_port: [u8; 2],
    pub seq_num: [u8; 4],
    pub ack_num: [u8; 4],
    pub data_offset_flags: [u8; 2],
    pub window: [u8; 2],
    pub checksum: [u8; 2],
    pub urgent_ptr: [u8; 2],
}

impl TcpHeader {
    pub fn source_port(&self) -> u16 {
        u16::from_be_bytes(self.src_port)
    }
    
    pub fn destination_port(&self) -> u16 {
        u16::from_be_bytes(self.dst_port)
    }
    
    pub fn sequence_number(&self) -> u32 {
        u32::from_be_bytes(self.seq_num)
    }
    
    pub fn acknowledgment_number(&self) -> u32 {
        u32::from_be_bytes(self.ack_num)
    }
    
    pub fn data_offset(&self) -> usize {
        ((self.data_offset_flags[0] >> 4) as usize) * 4
    }
    
    pub fn flags(&self) -> u8 {
        self.data_offset_flags[1]
    }
    
    pub fn window_size(&self) -> u16 {
        u16::from_be_bytes(self.window)
    }
}

/// TCP connection key
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TcpConnectionKey {
    pub local_addr: Ipv4Address,
    pub local_port: u16,
    pub remote_addr: Ipv4Address,
    pub remote_port: u16,
}

/// TCP Control Block
pub struct TcpControlBlock {
    /// Connection state
    pub state: TcpState,
    
    /// Local address
    pub local_addr: Ipv4Address,
    pub local_port: u16,
    
    /// Remote address
    pub remote_addr: Ipv4Address,
    pub remote_port: u16,
    
    /// Send sequence number
    pub snd_nxt: u32,
    /// Send unacknowledged
    pub snd_una: u32,
    /// Send window
    pub snd_wnd: u16,
    /// Initial send sequence number
    pub iss: u32,
    
    /// Receive next
    pub rcv_nxt: u32,
    /// Receive window
    pub rcv_wnd: u16,
    /// Initial receive sequence number
    pub irs: u32,
    
    /// Send buffer
    pub send_buf: VecDeque<u8>,
    /// Receive buffer
    pub recv_buf: VecDeque<u8>,
}

impl TcpControlBlock {
    pub fn new() -> Self {
        TcpControlBlock {
            state: TcpState::Closed,
            local_addr: Ipv4Address::UNSPECIFIED,
            local_port: 0,
            remote_addr: Ipv4Address::UNSPECIFIED,
            remote_port: 0,
            snd_nxt: 0,
            snd_una: 0,
            snd_wnd: 65535,
            iss: 0,
            rcv_nxt: 0,
            rcv_wnd: 65535,
            irs: 0,
            send_buf: VecDeque::new(),
            recv_buf: VecDeque::new(),
        }
    }
}

lazy_static! {
    /// Active TCP connections
    static ref TCP_CONNECTIONS: Mutex<BTreeMap<TcpConnectionKey, TcpControlBlock>> = 
        Mutex::new(BTreeMap::new());
    
    /// Next ephemeral port
    static ref NEXT_PORT: Mutex<u16> = Mutex::new(49152);
}

/// Allocate an ephemeral port
pub fn alloc_port() -> u16 {
    let mut port = NEXT_PORT.lock();
    let p = *port;
    *port = if *port >= 65535 { 49152 } else { *port + 1 };
    p
}

/// Process TCP packet
pub fn process_packet(src_ip: Ipv4Address, dst_ip: Ipv4Address, data: &[u8]) {
    if data.len() < TCP_HEADER_MIN {
        return;
    }
    
    let header = unsafe { &*(data.as_ptr() as *const TcpHeader) };
    let data_offset = header.data_offset();
    
    if data.len() < data_offset {
        return;
    }
    
    let _payload = &data[data_offset..];
    let flags = header.flags();
    
    let key = TcpConnectionKey {
        local_addr: dst_ip,
        local_port: header.destination_port(),
        remote_addr: src_ip,
        remote_port: header.source_port(),
    };
    
    let mut connections = TCP_CONNECTIONS.lock();
    
    if let Some(tcb) = connections.get_mut(&key) {
        // Existing connection
        process_segment(tcb, header, flags, _payload);
    } else if (flags & TcpFlags::SYN) != 0 && (flags & TcpFlags::ACK) == 0 {
        // New connection request - would check for listening socket
        crate::println!("TCP SYN from {}:{} to port {}", 
            src_ip, header.source_port(), header.destination_port());
    }
}

/// Process TCP segment for existing connection
fn process_segment(tcb: &mut TcpControlBlock, header: &TcpHeader, flags: u8, payload: &[u8]) {
    match tcb.state {
        TcpState::SynSent => {
            if (flags & TcpFlags::SYN) != 0 && (flags & TcpFlags::ACK) != 0 {
                tcb.rcv_nxt = header.sequence_number().wrapping_add(1);
                tcb.irs = header.sequence_number();
                tcb.snd_una = header.acknowledgment_number();
                tcb.state = TcpState::Established;
            }
        }
        TcpState::Established => {
            // Handle incoming data
            if !payload.is_empty() {
                tcb.recv_buf.extend(payload);
                tcb.rcv_nxt = tcb.rcv_nxt.wrapping_add(payload.len() as u32);
            }
            
            // Handle FIN
            if (flags & TcpFlags::FIN) != 0 {
                tcb.rcv_nxt = tcb.rcv_nxt.wrapping_add(1);
                tcb.state = TcpState::CloseWait;
            }
        }
        _ => {}
    }
}

/// Create TCP packet
pub fn create_packet(
    src_ip: Ipv4Address,
    dst_ip: Ipv4Address,
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
    flags: u8,
    window: u16,
    payload: &[u8],
) -> alloc::vec::Vec<u8> {
    let header_len = TCP_HEADER_MIN;
    let mut packet = alloc::vec![0u8; header_len + payload.len()];
    
    // Source port
    packet[0..2].copy_from_slice(&src_port.to_be_bytes());
    // Destination port
    packet[2..4].copy_from_slice(&dst_port.to_be_bytes());
    // Sequence number
    packet[4..8].copy_from_slice(&seq.to_be_bytes());
    // Acknowledgment number
    packet[8..12].copy_from_slice(&ack.to_be_bytes());
    // Data offset (5 = 20 bytes) + reserved
    packet[12] = 0x50;
    // Flags
    packet[13] = flags;
    // Window
    packet[14..16].copy_from_slice(&window.to_be_bytes());
    // Checksum (initially 0)
    packet[16..18].copy_from_slice(&0u16.to_be_bytes());
    // Urgent pointer
    packet[18..20].copy_from_slice(&0u16.to_be_bytes());
    // Payload
    if !payload.is_empty() {
        packet[header_len..].copy_from_slice(payload);
    }
    
    // Calculate checksum
    let cksum = tcp_checksum(src_ip, dst_ip, &packet);
    packet[16..18].copy_from_slice(&cksum.to_be_bytes());
    
    packet
}

/// Calculate TCP checksum with pseudo-header
fn tcp_checksum(src_ip: Ipv4Address, dst_ip: Ipv4Address, packet: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    
    // Pseudo-header
    sum += u16::from_be_bytes([src_ip.0[0], src_ip.0[1]]) as u32;
    sum += u16::from_be_bytes([src_ip.0[2], src_ip.0[3]]) as u32;
    sum += u16::from_be_bytes([dst_ip.0[0], dst_ip.0[1]]) as u32;
    sum += u16::from_be_bytes([dst_ip.0[2], dst_ip.0[3]]) as u32;
    sum += 6u32;  // Protocol = TCP
    sum += packet.len() as u32;
    
    // TCP packet
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
    
    !sum as u16
}

