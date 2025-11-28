//! Device Tree
//!
//! Structures for representing devices in the system.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use super::{DeviceClass, BusType, DriverId};
use super::resources::DeviceResources;

/// Unique device identifier
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(pub u64);

impl fmt::Debug for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "dev:{}", self.0)
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Device state in the lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// Device discovered but not registered
    Discovered,
    
    /// Device registered in device tree
    Registered,
    
    /// Driver bound to device
    Bound,
    
    /// Device initialized and ready
    Ready,
    
    /// Device suspended (power management)
    Suspended,
    
    /// Device has an error
    Error,
    
    /// Device removed
    Removed,
}

/// A device in the device tree
#[derive(Debug)]
pub struct Device {
    /// Unique device ID
    pub id: DeviceId,
    
    /// Device name (e.g., "usb-hid-keyboard")
    pub name: String,
    
    /// Device class
    pub class: DeviceClass,
    
    /// Bus this device is attached to
    pub bus: BusType,
    
    /// Parent device (None for root)
    pub parent: Option<DeviceId>,
    
    /// Child devices
    pub children: Vec<DeviceId>,
    
    /// Device state
    pub state: DeviceState,
    
    /// Assigned driver
    pub driver: Option<DriverId>,
    
    /// Device resources (MMIO, IRQ, etc.)
    pub resources: DeviceResources,
    
    /// Vendor ID (for PCI/USB)
    pub vendor_id: Option<u16>,
    
    /// Device ID (for PCI/USB)
    pub device_id: Option<u16>,
    
    /// Device-specific data
    pub private_data: Option<usize>,
}

impl Device {
    /// Create a new device
    pub fn new(
        id: DeviceId,
        name: &str,
        class: DeviceClass,
        bus: BusType,
        parent: Option<DeviceId>,
    ) -> Self {
        Device {
            id,
            name: String::from(name),
            class,
            bus,
            parent,
            children: Vec::new(),
            state: DeviceState::Registered,
            driver: None,
            resources: DeviceResources::empty(),
            vendor_id: None,
            device_id: None,
            private_data: None,
        }
    }
    
    /// Create a PCI device
    pub fn new_pci(
        id: DeviceId,
        name: &str,
        class: DeviceClass,
        vendor: u16,
        device: u16,
        parent: Option<DeviceId>,
    ) -> Self {
        let mut dev = Self::new(id, name, class, BusType::Pci, parent);
        dev.vendor_id = Some(vendor);
        dev.device_id = Some(device);
        dev
    }
    
    /// Create a USB device
    pub fn new_usb(
        id: DeviceId,
        name: &str,
        class: DeviceClass,
        vendor: u16,
        product: u16,
        parent: Option<DeviceId>,
    ) -> Self {
        let mut dev = Self::new(id, name, class, BusType::Usb, parent);
        dev.vendor_id = Some(vendor);
        dev.device_id = Some(product);
        dev
    }
    
    /// Check if device has a driver
    pub fn has_driver(&self) -> bool {
        self.driver.is_some()
    }
    
    /// Check if device is ready
    pub fn is_ready(&self) -> bool {
        self.state == DeviceState::Ready
    }
    
    /// Add MMIO resource
    pub fn add_mmio(&mut self, base: usize, size: usize) {
        self.resources.add_mmio(base, size);
    }
    
    /// Add IRQ resource
    pub fn add_irq(&mut self, irq: u32) {
        self.resources.add_irq(irq);
    }
    
    /// Add I/O port resource (x86 only)
    pub fn add_io_port(&mut self, base: u16, size: u16) {
        self.resources.add_io_port(base, size);
    }
}

