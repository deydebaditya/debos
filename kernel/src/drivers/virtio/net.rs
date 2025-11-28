//! VirtIO Network Driver
//!
//! Implements VirtIO-Net device for network connectivity.

use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;

use super::mmio::MmioDevice;
use super::queue::VirtQueue;
use crate::drivers::net::{MacAddress, Ipv4Address, NetworkInterface};

/// VirtIO network device type
pub const VIRTIO_NET_DEVICE_ID: u32 = 1;

/// VirtIO network feature bits
pub mod Features {
    pub const VIRTIO_NET_F_CSUM: u64 = 1 << 0;
    pub const VIRTIO_NET_F_GUEST_CSUM: u64 = 1 << 1;
    pub const VIRTIO_NET_F_MAC: u64 = 1 << 5;
    pub const VIRTIO_NET_F_STATUS: u64 = 1 << 16;
    pub const VIRTIO_NET_F_MRG_RXBUF: u64 = 1 << 15;
}

/// VirtIO network header
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VirtioNetHeader {
    pub flags: u8,
    pub gso_type: u8,
    pub hdr_len: u16,
    pub gso_size: u16,
    pub csum_start: u16,
    pub csum_offset: u16,
    pub num_buffers: u16,
}

impl VirtioNetHeader {
    pub const SIZE: usize = 12;
    
    pub fn new() -> Self {
        VirtioNetHeader::default()
    }
}

/// VirtIO network device
pub struct VirtioNet {
    /// MMIO device
    mmio: MmioDevice,
    
    /// Receive queue
    rx_queue: VirtQueue,
    
    /// Transmit queue
    tx_queue: VirtQueue,
    
    /// MAC address
    mac: MacAddress,
    
    /// Is device ready
    ready: bool,
    
    /// Receive buffers
    rx_buffers: Vec<Vec<u8>>,
    
    /// Statistics
    rx_packets: u64,
    tx_packets: u64,
    rx_bytes: u64,
    tx_bytes: u64,
}

/// RX buffer size
const RX_BUFFER_SIZE: usize = 2048;
/// Number of RX buffers
const RX_BUFFER_COUNT: usize = 16;

lazy_static! {
    /// Global VirtIO network device
    pub static ref VIRTIO_NET: Mutex<Option<VirtioNet>> = Mutex::new(None);
}

impl VirtioNet {
    /// Create a new VirtIO network device
    pub fn new(base_addr: usize) -> Result<Self, &'static str> {
        let mmio = MmioDevice::probe(base_addr)?;
        
        // Check device type
        let device_id = mmio.device_id();
        if device_id != VIRTIO_NET_DEVICE_ID {
            return Err("Not a VirtIO network device");
        }
        
        // Create queues
        let rx_queue = VirtQueue::new(0, 64);
        let tx_queue = VirtQueue::new(1, 64);
        
        // Read MAC address from config space
        let mac = Self::read_mac(&mmio);
        
        Ok(VirtioNet {
            mmio,
            rx_queue,
            tx_queue,
            mac,
            ready: false,
            rx_buffers: Vec::new(),
            rx_packets: 0,
            tx_packets: 0,
            rx_bytes: 0,
            tx_bytes: 0,
        })
    }
    
    /// Read MAC address from device config
    fn read_mac(mmio: &MmioDevice) -> MacAddress {
        let mut mac = [0u8; 6];
        for i in 0..6 {
            mac[i] = mmio.read_config_u8(i);
        }
        MacAddress(mac)
    }
    
    /// Initialize the device
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Reset device
        self.mmio.reset();
        
        // Acknowledge device
        self.mmio.set_status(0x01); // ACKNOWLEDGE
        self.mmio.set_status(0x03); // DRIVER
        
        // Negotiate features
        let features = self.mmio.read_features();
        let supported = Features::VIRTIO_NET_F_MAC | Features::VIRTIO_NET_F_STATUS;
        self.mmio.write_features(features & supported);
        
        // Features OK
        self.mmio.set_status(0x0B); // FEATURES_OK
        
        // Initialize queues
        self.mmio.select_queue(0);
        self.mmio.set_queue_size(64);
        self.mmio.set_queue_desc(self.rx_queue.desc_addr());
        self.mmio.set_queue_avail(self.rx_queue.avail_addr());
        self.mmio.set_queue_used(self.rx_queue.used_addr());
        self.mmio.enable_queue();
        
        self.mmio.select_queue(1);
        self.mmio.set_queue_size(64);
        self.mmio.set_queue_desc(self.tx_queue.desc_addr());
        self.mmio.set_queue_avail(self.tx_queue.avail_addr());
        self.mmio.set_queue_used(self.tx_queue.used_addr());
        self.mmio.enable_queue();
        
        // Set up receive buffers
        self.setup_rx_buffers()?;
        
        // Driver ready
        self.mmio.set_status(0x0F); // DRIVER_OK
        
        self.ready = true;
        Ok(())
    }
    
    /// Set up receive buffers
    fn setup_rx_buffers(&mut self) -> Result<(), &'static str> {
        for _ in 0..RX_BUFFER_COUNT {
            let buffer = alloc::vec![0u8; RX_BUFFER_SIZE];
            let addr = buffer.as_ptr() as u64;
            
            // Add to RX queue
            self.rx_queue.add_buffer(&[addr], &[], RX_BUFFER_SIZE as u32)?;
            self.rx_buffers.push(buffer);
        }
        
        // Notify device about available buffers
        self.mmio.notify(0);
        
        Ok(())
    }
    
    /// Get MAC address
    pub fn mac(&self) -> MacAddress {
        self.mac
    }
    
    /// Check if device is ready
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    
    /// Send a packet
    pub fn send(&mut self, data: &[u8]) -> Result<(), &'static str> {
        if !self.ready {
            return Err("Device not ready");
        }
        
        // Create packet with virtio header
        let mut packet = alloc::vec![0u8; VirtioNetHeader::SIZE + data.len()];
        packet[VirtioNetHeader::SIZE..].copy_from_slice(data);
        
        let addr = packet.as_ptr() as u64;
        let len = packet.len() as u32;
        
        // Add to TX queue
        self.tx_queue.add_buffer(&[addr], &[], len)?;
        
        // Notify device
        self.mmio.notify(1);
        
        self.tx_packets += 1;
        self.tx_bytes += data.len() as u64;
        
        Ok(())
    }
    
    /// Receive packets (polling)
    pub fn recv(&mut self) -> Option<Vec<u8>> {
        if !self.ready {
            return None;
        }
        
        // Check for completed RX buffers
        if let Some((idx, len)) = self.rx_queue.pop_used() {
            if idx < self.rx_buffers.len() {
                let buffer = &self.rx_buffers[idx];
                let len = len as usize;
                
                if len > VirtioNetHeader::SIZE {
                    let data_len = len - VirtioNetHeader::SIZE;
                    let mut data = alloc::vec![0u8; data_len];
                    data.copy_from_slice(&buffer[VirtioNetHeader::SIZE..len]);
                    
                    self.rx_packets += 1;
                    self.rx_bytes += data_len as u64;
                    
                    // Re-add buffer to queue
                    let addr = buffer.as_ptr() as u64;
                    let _ = self.rx_queue.add_buffer(&[addr], &[], RX_BUFFER_SIZE as u32);
                    self.mmio.notify(0);
                    
                    return Some(data);
                }
            }
        }
        
        None
    }
    
    /// Get statistics
    pub fn stats(&self) -> (u64, u64, u64, u64) {
        (self.rx_packets, self.tx_packets, self.rx_bytes, self.tx_bytes)
    }
}

/// Initialize VirtIO network driver
pub fn init() -> Result<(), &'static str> {
    // Scan for VirtIO network device at known MMIO addresses
    let mmio_addrs: &[usize] = &[
        0x0A003E00,  // QEMU virt machine VirtIO MMIO
        0x0A003C00,
        0x0A003A00,
    ];
    
    for &addr in mmio_addrs {
        if let Ok(mut net) = VirtioNet::new(addr) {
            if net.init().is_ok() {
                let mac = net.mac();
                crate::println!("    VirtIO-Net: {} at {:#x}", mac, addr);
                
                // Register network interface
                let iface = NetworkInterface::new("eth0", mac);
                crate::drivers::net::register_interface(iface);
                
                // Configure default IP (QEMU user-mode networking)
                let _ = crate::drivers::net::configure_interface(
                    "eth0",
                    Ipv4Address::new(10, 0, 2, 15),
                    Ipv4Address::new(255, 255, 255, 0),
                    Some(Ipv4Address::new(10, 0, 2, 2)),
                );
                
                *VIRTIO_NET.lock() = Some(net);
                return Ok(());
            }
        }
    }
    
    // No VirtIO network device found
    Ok(())
}

/// Send a packet through VirtIO-Net
pub fn send_packet(data: &[u8]) -> Result<(), &'static str> {
    let mut net = VIRTIO_NET.lock();
    match net.as_mut() {
        Some(n) => n.send(data),
        None => Err("No VirtIO network device"),
    }
}

/// Receive a packet from VirtIO-Net
pub fn recv_packet() -> Option<Vec<u8>> {
    let mut net = VIRTIO_NET.lock();
    net.as_mut().and_then(|n| n.recv())
}

/// Check if VirtIO-Net is available
pub fn is_available() -> bool {
    VIRTIO_NET.lock().is_some()
}

/// Get VirtIO-Net statistics
pub fn get_stats() -> Option<(u64, u64, u64, u64)> {
    VIRTIO_NET.lock().as_ref().map(|n| n.stats())
}

