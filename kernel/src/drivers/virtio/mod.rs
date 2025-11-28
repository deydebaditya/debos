//! VirtIO Subsystem
//!
//! Implementation of the VirtIO specification for paravirtualized devices.
//! VirtIO provides a standardized interface for virtual devices in QEMU/KVM.
//!
//! ## Supported Transports
//! - MMIO (Memory-Mapped I/O) - Used by QEMU's virt machine
//!
//! ## Supported Devices
//! - VirtIO-Block (Block storage)
//!
//! ## References
//! - VirtIO Specification: https://docs.oasis-open.org/virtio/virtio/v1.1/virtio-v1.1.html

pub mod queue;
pub mod mmio;
pub mod block;

use spin::Mutex;
use alloc::vec::Vec;

/// VirtIO device types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Invalid = 0,
    Network = 1,
    Block = 2,
    Console = 3,
    Entropy = 4,
    Balloon = 5,
    ScsiHost = 8,
    Gpu = 16,
    Input = 18,
    Socket = 19,
}

impl From<u32> for DeviceType {
    fn from(value: u32) -> Self {
        match value {
            1 => DeviceType::Network,
            2 => DeviceType::Block,
            3 => DeviceType::Console,
            4 => DeviceType::Entropy,
            5 => DeviceType::Balloon,
            8 => DeviceType::ScsiHost,
            16 => DeviceType::Gpu,
            18 => DeviceType::Input,
            19 => DeviceType::Socket,
            _ => DeviceType::Invalid,
        }
    }
}

/// VirtIO device status bits
pub mod status {
    pub const ACKNOWLEDGE: u32 = 1;
    pub const DRIVER: u32 = 2;
    pub const DRIVER_OK: u32 = 4;
    pub const FEATURES_OK: u32 = 8;
    pub const DEVICE_NEEDS_RESET: u32 = 64;
    pub const FAILED: u32 = 128;
}

/// VirtIO feature bits (common)
pub mod features {
    pub const RING_INDIRECT_DESC: u64 = 1 << 28;
    pub const RING_EVENT_IDX: u64 = 1 << 29;
    pub const VERSION_1: u64 = 1 << 32;
    pub const ACCESS_PLATFORM: u64 = 1 << 33;
    pub const RING_PACKED: u64 = 1 << 34;
}

/// List of discovered VirtIO devices
static DEVICES: Mutex<Vec<mmio::MmioDevice>> = Mutex::new(Vec::new());

/// Initialize the VirtIO subsystem
pub fn init() {
    crate::println!("  [..] Scanning for VirtIO devices...");
    
    // Probe for VirtIO MMIO devices
    let devices = mmio::probe_devices();
    
    if devices.is_empty() {
        crate::println!("  No VirtIO devices found at MMIO addresses");
        crate::println!("  (This is normal if no VirtIO devices are attached)");
    }
    
    for device in &devices {
        crate::println!("  Found VirtIO device: {:?} at 0x{:x}", 
            device.device_type(), device.base_addr());
            
        // Initialize block devices
        if device.device_type() == DeviceType::Block {
            if let Err(e) = block::init_device(device) {
                crate::println!("    Failed to initialize block device: {:?}", e);
            }
        }
    }
    
    *DEVICES.lock() = devices;
    
    crate::println!("  [OK] VirtIO initialized ({} devices)", DEVICES.lock().len());
}

/// Get the number of discovered devices
pub fn device_count() -> usize {
    DEVICES.lock().len()
}

