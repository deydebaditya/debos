//! Network Subsystem
//!
//! Provides TCP/IP networking with socket API.

pub mod ethernet;
pub mod arp;
pub mod ipv4;
pub mod icmp;
pub mod udp;
pub mod tcp;
pub mod socket;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

/// MAC address (6 bytes)
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    pub const BROADCAST: MacAddress = MacAddress([0xFF; 6]);
    pub const ZERO: MacAddress = MacAddress([0; 6]);
    
    pub fn new(bytes: [u8; 6]) -> Self {
        MacAddress(bytes)
    }
    
    pub fn is_broadcast(&self) -> bool {
        *self == Self::BROADCAST
    }
    
    pub fn is_multicast(&self) -> bool {
        (self.0[0] & 0x01) != 0
    }
}

impl core::fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5])
    }
}

impl core::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

/// IPv4 address
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    pub const UNSPECIFIED: Ipv4Address = Ipv4Address([0, 0, 0, 0]);
    pub const BROADCAST: Ipv4Address = Ipv4Address([255, 255, 255, 255]);
    pub const LOCALHOST: Ipv4Address = Ipv4Address([127, 0, 0, 1]);
    
    pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Ipv4Address([a, b, c, d])
    }
    
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Ipv4Address(bytes)
    }
    
    pub fn to_u32(&self) -> u32 {
        u32::from_be_bytes(self.0)
    }
    
    pub fn from_u32(val: u32) -> Self {
        Ipv4Address(val.to_be_bytes())
    }
    
    pub fn is_unspecified(&self) -> bool {
        *self == Self::UNSPECIFIED
    }
    
    pub fn is_broadcast(&self) -> bool {
        *self == Self::BROADCAST
    }
    
    pub fn is_loopback(&self) -> bool {
        self.0[0] == 127
    }
    
    pub fn is_private(&self) -> bool {
        // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
        self.0[0] == 10 ||
        (self.0[0] == 172 && (self.0[1] & 0xF0) == 16) ||
        (self.0[0] == 192 && self.0[1] == 168)
    }
}

impl core::fmt::Debug for Ipv4Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

impl core::fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

/// Parse IPv4 address from string
pub fn parse_ipv4(s: &str) -> Option<Ipv4Address> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    
    let mut bytes = [0u8; 4];
    for (i, part) in parts.iter().enumerate() {
        bytes[i] = part.parse().ok()?;
    }
    
    Some(Ipv4Address(bytes))
}

/// Network interface
pub struct NetworkInterface {
    /// Interface name (e.g., "eth0")
    pub name: String,
    
    /// MAC address
    pub mac: MacAddress,
    
    /// IPv4 address
    pub ipv4: Option<Ipv4Address>,
    
    /// Subnet mask
    pub netmask: Option<Ipv4Address>,
    
    /// Gateway address
    pub gateway: Option<Ipv4Address>,
    
    /// Interface is up
    pub up: bool,
    
    /// MTU
    pub mtu: u16,
    
    /// TX packet count
    pub tx_packets: u64,
    
    /// RX packet count
    pub rx_packets: u64,
    
    /// TX byte count
    pub tx_bytes: u64,
    
    /// RX byte count
    pub rx_bytes: u64,
}

impl NetworkInterface {
    pub fn new(name: &str, mac: MacAddress) -> Self {
        NetworkInterface {
            name: String::from(name),
            mac,
            ipv4: None,
            netmask: None,
            gateway: None,
            up: false,
            mtu: 1500,
            tx_packets: 0,
            rx_packets: 0,
            tx_bytes: 0,
            rx_bytes: 0,
        }
    }
    
    /// Configure IPv4 address
    pub fn set_ipv4(&mut self, addr: Ipv4Address, netmask: Ipv4Address, gateway: Option<Ipv4Address>) {
        self.ipv4 = Some(addr);
        self.netmask = Some(netmask);
        self.gateway = gateway;
    }
    
    /// Bring interface up
    pub fn up(&mut self) {
        self.up = true;
    }
    
    /// Bring interface down
    pub fn down(&mut self) {
        self.up = false;
    }
}

lazy_static! {
    /// Network interfaces
    static ref INTERFACES: Mutex<BTreeMap<String, NetworkInterface>> = Mutex::new(BTreeMap::new());
}

/// Initialize networking subsystem
pub fn init() {
    crate::println!("  [..] Initializing network subsystem...");
    
    // Initialize ARP cache
    arp::init();
    
    // Initialize socket layer
    socket::init();
    
    // Create loopback interface
    let lo = NetworkInterface {
        name: String::from("lo"),
        mac: MacAddress::ZERO,
        ipv4: Some(Ipv4Address::LOCALHOST),
        netmask: Some(Ipv4Address::new(255, 0, 0, 0)),
        gateway: None,
        up: true,
        mtu: 65535,  // Maximum for u16
        tx_packets: 0,
        rx_packets: 0,
        tx_bytes: 0,
        rx_bytes: 0,
    };
    
    INTERFACES.lock().insert(String::from("lo"), lo);
    
    crate::println!("  [OK] Network subsystem initialized");
}

/// Register a network interface
pub fn register_interface(iface: NetworkInterface) {
    let name = iface.name.clone();
    INTERFACES.lock().insert(name, iface);
}

/// Get interface by name
pub fn get_interface(name: &str) -> Option<NetworkInterface> {
    INTERFACES.lock().get(name).cloned()
}

/// List all interfaces
pub fn list_interfaces() -> Vec<String> {
    INTERFACES.lock().keys().cloned().collect()
}

/// Configure interface
pub fn configure_interface(name: &str, ipv4: Ipv4Address, netmask: Ipv4Address, gateway: Option<Ipv4Address>) -> Result<(), &'static str> {
    let mut interfaces = INTERFACES.lock();
    let iface = interfaces.get_mut(name).ok_or("Interface not found")?;
    iface.set_ipv4(ipv4, netmask, gateway);
    iface.up();
    Ok(())
}

/// Send a packet through an interface
pub fn send_packet(iface_name: &str, data: &[u8]) -> Result<(), &'static str> {
    let mut interfaces = INTERFACES.lock();
    let iface = interfaces.get_mut(iface_name).ok_or("Interface not found")?;
    
    if !iface.up {
        return Err("Interface is down");
    }
    
    iface.tx_packets += 1;
    iface.tx_bytes += data.len() as u64;
    
    // Actual packet transmission would go here (driver call)
    
    Ok(())
}

/// Receive a packet (called from driver)
pub fn receive_packet(iface_name: &str, data: &[u8]) {
    let mut interfaces = INTERFACES.lock();
    if let Some(iface) = interfaces.get_mut(iface_name) {
        iface.rx_packets += 1;
        iface.rx_bytes += data.len() as u64;
    }
    drop(interfaces);
    
    // Process the packet through the network stack
    if data.len() >= 14 {
        ethernet::process_frame(data);
    }
}

impl Clone for NetworkInterface {
    fn clone(&self) -> Self {
        NetworkInterface {
            name: self.name.clone(),
            mac: self.mac,
            ipv4: self.ipv4,
            netmask: self.netmask,
            gateway: self.gateway,
            up: self.up,
            mtu: self.mtu,
            tx_packets: self.tx_packets,
            rx_packets: self.rx_packets,
            tx_bytes: self.tx_bytes,
            rx_bytes: self.rx_bytes,
        }
    }
}

