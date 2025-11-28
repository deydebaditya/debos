//! VirtIO MMIO Transport
//!
//! Memory-Mapped I/O transport for VirtIO devices.
//! Used by QEMU's virt machine for ARM64.
//!
//! ## Register Map (VirtIO v2, MMIO)
//! - 0x000: MagicValue (0x74726976 = "virt")
//! - 0x004: Version (2 for modern)
//! - 0x008: DeviceID (device type)
//! - 0x00C: VendorID
//! - 0x010: DeviceFeatures
//! - 0x014: DeviceFeaturesSel
//! - 0x020: DriverFeatures
//! - 0x024: DriverFeaturesSel
//! - 0x030: QueueSel
//! - 0x034: QueueNumMax
//! - 0x038: QueueNum
//! - 0x044: QueueReady
//! - 0x050: QueueNotify
//! - 0x060: InterruptStatus
//! - 0x064: InterruptACK
//! - 0x070: Status
//! - 0x080: QueueDescLow/High
//! - 0x090: QueueDriverLow/High
//! - 0x0A0: QueueDeviceLow/High
//! - 0x0FC: ConfigGeneration
//! - 0x100+: Device-specific config

use core::ptr::{read_volatile, write_volatile};
use alloc::vec::Vec;
use super::{DeviceType, status, features, queue::VirtQueue};

/// MMIO Register offsets
mod regs {
    pub const MAGIC_VALUE: usize = 0x000;
    pub const VERSION: usize = 0x004;
    pub const DEVICE_ID: usize = 0x008;
    pub const VENDOR_ID: usize = 0x00C;
    pub const DEVICE_FEATURES: usize = 0x010;
    pub const DEVICE_FEATURES_SEL: usize = 0x014;
    pub const DRIVER_FEATURES: usize = 0x020;
    pub const DRIVER_FEATURES_SEL: usize = 0x024;
    pub const QUEUE_SEL: usize = 0x030;
    pub const QUEUE_NUM_MAX: usize = 0x034;
    pub const QUEUE_NUM: usize = 0x038;
    pub const QUEUE_READY: usize = 0x044;
    pub const QUEUE_NOTIFY: usize = 0x050;
    pub const INTERRUPT_STATUS: usize = 0x060;
    pub const INTERRUPT_ACK: usize = 0x064;
    pub const STATUS: usize = 0x070;
    pub const QUEUE_DESC_LOW: usize = 0x080;
    pub const QUEUE_DESC_HIGH: usize = 0x084;
    pub const QUEUE_DRIVER_LOW: usize = 0x090;
    pub const QUEUE_DRIVER_HIGH: usize = 0x094;
    pub const QUEUE_DEVICE_LOW: usize = 0x0A0;
    pub const QUEUE_DEVICE_HIGH: usize = 0x0A4;
    pub const CONFIG_GENERATION: usize = 0x0FC;
    pub const CONFIG: usize = 0x100;
}

/// VirtIO MMIO magic value
const VIRTIO_MAGIC: u32 = 0x74726976; // "virt"

/// VirtIO MMIO versions we support
const VIRTIO_VERSION_LEGACY: u32 = 1;
const VIRTIO_VERSION_MODERN: u32 = 2;

/// QEMU virt machine VirtIO MMIO regions
/// On aarch64 virt, devices are at 0x0a00_0000 + 0x200 * n
#[cfg(target_arch = "aarch64")]
const VIRTIO_MMIO_REGIONS: &[(usize, u32)] = &[
    (0x0a000000, 0x30), // IRQ 48
    (0x0a000200, 0x31), // IRQ 49
    (0x0a000400, 0x32), // IRQ 50
    (0x0a000600, 0x33), // IRQ 51
    (0x0a000800, 0x34), // IRQ 52
    (0x0a000a00, 0x35), // IRQ 53
    (0x0a000c00, 0x36), // IRQ 54
    (0x0a000e00, 0x37), // IRQ 55
];

#[cfg(target_arch = "x86_64")]
const VIRTIO_MMIO_REGIONS: &[(usize, u32)] = &[];

/// A VirtIO MMIO device
#[derive(Debug)]
pub struct MmioDevice {
    base: usize,
    irq: u32,
    device_type: DeviceType,
    features: u64,
}

impl MmioDevice {
    /// Create a new MMIO device handle
    pub fn new(base: usize, irq: u32) -> Option<Self> {
        // Check magic value
        let magic = unsafe { read_volatile((base + regs::MAGIC_VALUE) as *const u32) };
        if magic != VIRTIO_MAGIC {
            return None;
        }
        
        // Check version (support both legacy and modern)
        let version = unsafe { read_volatile((base + regs::VERSION) as *const u32) };
        if version != VIRTIO_VERSION_LEGACY && version != VIRTIO_VERSION_MODERN {
            return None;
        }
        
        // Get device type
        let device_id = unsafe { read_volatile((base + regs::DEVICE_ID) as *const u32) };
        if device_id == 0 {
            return None; // No device at this slot
        }
        
        Some(MmioDevice {
            base,
            irq,
            device_type: DeviceType::from(device_id),
            features: 0,
        })
    }
    
    /// Create a new MMIO device handle (without IRQ, for probing)
    pub fn probe(base: usize) -> Result<Self, &'static str> {
        Self::new(base, 0).ok_or("No VirtIO device at this address")
    }
    
    /// Get device ID
    pub fn device_id(&self) -> u32 {
        match self.device_type {
            DeviceType::Network => 1,
            DeviceType::Block => 2,
            DeviceType::Console => 3,
            DeviceType::Entropy => 4,
            DeviceType::Balloon => 5,
            DeviceType::ScsiHost => 8,
            DeviceType::Gpu => 16,
            DeviceType::Input => 18,
            DeviceType::Socket => 19,
            DeviceType::Invalid => 0,
        }
    }
    
    /// Set device status (replaces current status)
    pub fn set_status(&self, status: u32) {
        self.write_reg(regs::STATUS, status);
    }
    
    /// Select a queue
    pub fn select_queue(&self, idx: u16) {
        self.write_reg(regs::QUEUE_SEL, idx as u32);
    }
    
    /// Set queue size
    pub fn set_queue_size(&self, size: u16) {
        self.write_reg(regs::QUEUE_NUM, size as u32);
    }
    
    /// Set queue descriptor address
    pub fn set_queue_desc(&self, addr: u64) {
        self.write_reg(regs::QUEUE_DESC_LOW, addr as u32);
        self.write_reg(regs::QUEUE_DESC_HIGH, (addr >> 32) as u32);
    }
    
    /// Set queue available ring address
    pub fn set_queue_avail(&self, addr: u64) {
        self.write_reg(regs::QUEUE_DRIVER_LOW, addr as u32);
        self.write_reg(regs::QUEUE_DRIVER_HIGH, (addr >> 32) as u32);
    }
    
    /// Set queue used ring address
    pub fn set_queue_used(&self, addr: u64) {
        self.write_reg(regs::QUEUE_DEVICE_LOW, addr as u32);
        self.write_reg(regs::QUEUE_DEVICE_HIGH, (addr >> 32) as u32);
    }
    
    /// Enable the current queue
    pub fn enable_queue(&self) {
        self.write_reg(regs::QUEUE_READY, 1);
    }
    
    /// Notify device about queue activity
    pub fn notify(&self, queue_idx: u16) {
        self.write_reg(regs::QUEUE_NOTIFY, queue_idx as u32);
    }
    
    /// Get base address
    pub fn base_addr(&self) -> usize {
        self.base
    }
    
    /// Get IRQ number
    pub fn irq(&self) -> u32 {
        self.irq
    }
    
    /// Get device type
    pub fn device_type(&self) -> DeviceType {
        self.device_type
    }
    
    /// Read a register
    fn read_reg(&self, offset: usize) -> u32 {
        unsafe { read_volatile((self.base + offset) as *const u32) }
    }
    
    /// Write a register
    fn write_reg(&self, offset: usize, value: u32) {
        unsafe { write_volatile((self.base + offset) as *mut u32, value) }
    }
    
    /// Read device features
    pub fn read_features(&self) -> u64 {
        self.write_reg(regs::DEVICE_FEATURES_SEL, 0);
        let low = self.read_reg(regs::DEVICE_FEATURES) as u64;
        
        self.write_reg(regs::DEVICE_FEATURES_SEL, 1);
        let high = self.read_reg(regs::DEVICE_FEATURES) as u64;
        
        low | (high << 32)
    }
    
    /// Write driver features
    pub fn write_features(&mut self, features: u64) {
        self.write_reg(regs::DRIVER_FEATURES_SEL, 0);
        self.write_reg(regs::DRIVER_FEATURES, features as u32);
        
        self.write_reg(regs::DRIVER_FEATURES_SEL, 1);
        self.write_reg(regs::DRIVER_FEATURES, (features >> 32) as u32);
        
        self.features = features;
    }
    
    /// Get the negotiated features
    pub fn features(&self) -> u64 {
        self.features
    }
    
    /// Read device status
    pub fn read_status(&self) -> u32 {
        self.read_reg(regs::STATUS)
    }
    
    /// Write device status
    pub fn write_status(&self, status: u32) {
        self.write_reg(regs::STATUS, status);
    }
    
    /// Add a status flag
    pub fn add_status(&self, flag: u32) {
        let current = self.read_status();
        self.write_status(current | flag);
    }
    
    /// Reset the device
    pub fn reset(&self) {
        self.write_status(0);
    }
    
    /// Get maximum queue size for a queue
    pub fn queue_max_size(&self, queue_idx: u16) -> u16 {
        self.write_reg(regs::QUEUE_SEL, queue_idx as u32);
        self.read_reg(regs::QUEUE_NUM_MAX) as u16
    }
    
    /// Set up a queue
    pub fn setup_queue(&self, queue: &VirtQueue) {
        let idx = queue.index;
        
        // Select queue
        self.write_reg(regs::QUEUE_SEL, idx as u32);
        
        // Set queue size
        self.write_reg(regs::QUEUE_NUM, queue.size as u32);
        
        // Set descriptor area
        let desc_addr = queue.descriptor_area();
        self.write_reg(regs::QUEUE_DESC_LOW, desc_addr as u32);
        self.write_reg(regs::QUEUE_DESC_HIGH, (desc_addr >> 32) as u32);
        
        // Set driver (available) area
        let driver_addr = queue.driver_area();
        self.write_reg(regs::QUEUE_DRIVER_LOW, driver_addr as u32);
        self.write_reg(regs::QUEUE_DRIVER_HIGH, (driver_addr >> 32) as u32);
        
        // Set device (used) area
        let device_addr = queue.device_area();
        self.write_reg(regs::QUEUE_DEVICE_LOW, device_addr as u32);
        self.write_reg(regs::QUEUE_DEVICE_HIGH, (device_addr >> 32) as u32);
        
        // Enable queue
        self.write_reg(regs::QUEUE_READY, 1);
    }
    
    /// Notify the device about a queue
    pub fn notify_queue(&self, queue_idx: u16) {
        self.write_reg(regs::QUEUE_NOTIFY, queue_idx as u32);
    }
    
    /// Read interrupt status
    pub fn interrupt_status(&self) -> u32 {
        self.read_reg(regs::INTERRUPT_STATUS)
    }
    
    /// Acknowledge interrupt
    pub fn ack_interrupt(&self, status: u32) {
        self.write_reg(regs::INTERRUPT_ACK, status);
    }
    
    /// Read a config byte
    pub fn read_config_u8(&self, offset: usize) -> u8 {
        unsafe { read_volatile((self.base + regs::CONFIG + offset) as *const u8) }
    }
    
    /// Read a config u32
    pub fn read_config_u32(&self, offset: usize) -> u32 {
        unsafe { read_volatile((self.base + regs::CONFIG + offset) as *const u32) }
    }
    
    /// Read a config u64
    pub fn read_config_u64(&self, offset: usize) -> u64 {
        unsafe { read_volatile((self.base + regs::CONFIG + offset) as *const u64) }
    }
    
    /// Perform standard device initialization
    /// Returns device features on success
    pub fn init_device(&mut self, required_features: u64) -> Result<u64, &'static str> {
        // Reset device
        self.reset();
        
        // Acknowledge device
        self.add_status(status::ACKNOWLEDGE);
        
        // Driver knows how to drive the device
        self.add_status(status::DRIVER);
        
        // Read device features
        let device_features = self.read_features();
        
        // Negotiate features
        let negotiated = device_features & (required_features | features::VERSION_1);
        self.write_features(negotiated);
        
        // Set FEATURES_OK
        self.add_status(status::FEATURES_OK);
        
        // Check if device accepted features
        let status = self.read_status();
        if status & status::FEATURES_OK == 0 {
            return Err("Device did not accept features");
        }
        
        Ok(negotiated)
    }
    
    /// Mark device as ready
    pub fn finish_init(&self) {
        self.add_status(status::DRIVER_OK);
    }
}

/// Probe for VirtIO MMIO devices
pub fn probe_devices() -> Vec<MmioDevice> {
    let mut devices = Vec::new();
    
    for &(base, irq) in VIRTIO_MMIO_REGIONS {
        if let Some(device) = MmioDevice::new(base, irq) {
            devices.push(device);
        }
    }
    
    devices
}

