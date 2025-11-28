//! ARP Protocol
//!
//! Address Resolution Protocol for mapping IP to MAC addresses.

use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

use super::{MacAddress, Ipv4Address};

/// ARP operation codes
pub mod ArpOp {
    pub const REQUEST: u16 = 1;
    pub const REPLY: u16 = 2;
}

/// ARP header (for Ethernet/IPv4)
#[repr(C, packed)]
pub struct ArpHeader {
    /// Hardware type (1 = Ethernet)
    pub htype: [u8; 2],
    /// Protocol type (0x0800 = IPv4)
    pub ptype: [u8; 2],
    /// Hardware address length (6 for Ethernet)
    pub hlen: u8,
    /// Protocol address length (4 for IPv4)
    pub plen: u8,
    /// Operation (1 = request, 2 = reply)
    pub oper: [u8; 2],
    /// Sender hardware address
    pub sha: [u8; 6],
    /// Sender protocol address
    pub spa: [u8; 4],
    /// Target hardware address
    pub tha: [u8; 6],
    /// Target protocol address
    pub tpa: [u8; 4],
}

/// ARP cache entry
#[derive(Clone)]
pub struct ArpEntry {
    pub mac: MacAddress,
    pub timestamp: u64,
    pub state: ArpState,
}

/// ARP entry state
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ArpState {
    /// Entry is valid
    Valid,
    /// Waiting for reply
    Pending,
    /// Entry has expired
    Stale,
}

lazy_static! {
    /// ARP cache
    static ref ARP_CACHE: Mutex<BTreeMap<Ipv4Address, ArpEntry>> = Mutex::new(BTreeMap::new());
}

/// ARP cache timeout (5 minutes)
const ARP_TIMEOUT: u64 = 300 * 1000; // milliseconds

/// Initialize ARP
pub fn init() {
    // Nothing to do for now
}

/// Look up MAC address for IP
pub fn lookup(ip: Ipv4Address) -> Option<MacAddress> {
    let cache = ARP_CACHE.lock();
    if let Some(entry) = cache.get(&ip) {
        if entry.state == ArpState::Valid {
            return Some(entry.mac);
        }
    }
    None
}

/// Add entry to ARP cache
pub fn add_entry(ip: Ipv4Address, mac: MacAddress) {
    let entry = ArpEntry {
        mac,
        timestamp: crate::scheduler::ticks(),
        state: ArpState::Valid,
    };
    ARP_CACHE.lock().insert(ip, entry);
}

/// Remove entry from ARP cache
pub fn remove_entry(ip: Ipv4Address) {
    ARP_CACHE.lock().remove(&ip);
}

/// Process ARP packet
pub fn process_packet(data: &[u8]) {
    if data.len() < core::mem::size_of::<ArpHeader>() {
        return;
    }
    
    let header = unsafe { &*(data.as_ptr() as *const ArpHeader) };
    
    // Check for Ethernet/IPv4
    let htype = u16::from_be_bytes(header.htype);
    let ptype = u16::from_be_bytes(header.ptype);
    
    if htype != 1 || ptype != 0x0800 {
        return;
    }
    
    let oper = u16::from_be_bytes(header.oper);
    let sender_mac = MacAddress(header.sha);
    let sender_ip = Ipv4Address(header.spa);
    let _target_ip = Ipv4Address(header.tpa);
    
    // Update ARP cache with sender info
    add_entry(sender_ip, sender_mac);
    
    match oper {
        ArpOp::REQUEST => {
            // Check if we're the target
            // Would send reply here
        }
        ArpOp::REPLY => {
            // Already added to cache above
        }
        _ => {}
    }
}

/// Create ARP request packet
pub fn create_request(
    sender_mac: MacAddress,
    sender_ip: Ipv4Address,
    target_ip: Ipv4Address,
) -> alloc::vec::Vec<u8> {
    let mut packet = alloc::vec![0u8; 28];
    
    // Hardware type = Ethernet
    packet[0..2].copy_from_slice(&1u16.to_be_bytes());
    // Protocol type = IPv4
    packet[2..4].copy_from_slice(&0x0800u16.to_be_bytes());
    // Hardware length
    packet[4] = 6;
    // Protocol length
    packet[5] = 4;
    // Operation = Request
    packet[6..8].copy_from_slice(&ArpOp::REQUEST.to_be_bytes());
    // Sender hardware address
    packet[8..14].copy_from_slice(&sender_mac.0);
    // Sender protocol address
    packet[14..18].copy_from_slice(&sender_ip.0);
    // Target hardware address (unknown)
    packet[18..24].copy_from_slice(&[0u8; 6]);
    // Target protocol address
    packet[24..28].copy_from_slice(&target_ip.0);
    
    packet
}

/// Create ARP reply packet
pub fn create_reply(
    sender_mac: MacAddress,
    sender_ip: Ipv4Address,
    target_mac: MacAddress,
    target_ip: Ipv4Address,
) -> alloc::vec::Vec<u8> {
    let mut packet = alloc::vec![0u8; 28];
    
    packet[0..2].copy_from_slice(&1u16.to_be_bytes());
    packet[2..4].copy_from_slice(&0x0800u16.to_be_bytes());
    packet[4] = 6;
    packet[5] = 4;
    packet[6..8].copy_from_slice(&ArpOp::REPLY.to_be_bytes());
    packet[8..14].copy_from_slice(&sender_mac.0);
    packet[14..18].copy_from_slice(&sender_ip.0);
    packet[18..24].copy_from_slice(&target_mac.0);
    packet[24..28].copy_from_slice(&target_ip.0);
    
    packet
}

/// List ARP cache entries
pub fn list_cache() -> alloc::vec::Vec<(Ipv4Address, MacAddress)> {
    ARP_CACHE.lock()
        .iter()
        .filter(|(_, entry)| entry.state == ArpState::Valid)
        .map(|(ip, entry)| (*ip, entry.mac))
        .collect()
}

