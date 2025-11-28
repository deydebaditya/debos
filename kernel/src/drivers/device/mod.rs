//! Device Manager
//!
//! Central hub for device enumeration, driver management, and device tree.

pub mod tree;
pub mod resources;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

pub use tree::{Device, DeviceId, DeviceState};
pub use resources::{DeviceResources, MmioRegion, IoPortRange, DmaBuffer};

/// Device class categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    // Root
    Root,
    
    // Storage
    BlockDevice,
    
    // Input
    Keyboard,
    Mouse,
    Touchpad,
    Gamepad,
    GenericInput,
    
    // Display
    DisplayController,
    Framebuffer,
    
    // Network
    Ethernet,
    Wireless,
    
    // USB
    UsbController,
    UsbHub,
    UsbDevice,
    
    // Serial
    SerialPort,
    
    // Audio
    AudioController,
    
    // Other
    Unknown(u16),
}

impl DeviceClass {
    /// Get a human-readable name for the device class
    pub fn name(&self) -> &'static str {
        match self {
            DeviceClass::Root => "root",
            DeviceClass::BlockDevice => "block",
            DeviceClass::Keyboard => "keyboard",
            DeviceClass::Mouse => "mouse",
            DeviceClass::Touchpad => "touchpad",
            DeviceClass::Gamepad => "gamepad",
            DeviceClass::GenericInput => "input",
            DeviceClass::DisplayController => "display",
            DeviceClass::Framebuffer => "framebuffer",
            DeviceClass::Ethernet => "ethernet",
            DeviceClass::Wireless => "wireless",
            DeviceClass::UsbController => "usb-controller",
            DeviceClass::UsbHub => "usb-hub",
            DeviceClass::UsbDevice => "usb-device",
            DeviceClass::SerialPort => "serial",
            DeviceClass::AudioController => "audio",
            DeviceClass::Unknown(_) => "unknown",
        }
    }
}

/// Bus types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusType {
    /// Virtual root bus
    Root,
    /// PCI/PCIe bus
    Pci,
    /// USB bus
    Usb,
    /// Platform/SoC devices
    Platform,
    /// VirtIO (MMIO or PCI)
    VirtIO,
    /// I2C bus
    I2c,
    /// SPI bus
    Spi,
}

impl BusType {
    /// Get bus type name
    pub fn name(&self) -> &'static str {
        match self {
            BusType::Root => "root",
            BusType::Pci => "pci",
            BusType::Usb => "usb",
            BusType::Platform => "platform",
            BusType::VirtIO => "virtio",
            BusType::I2c => "i2c",
            BusType::Spi => "spi",
        }
    }
}

/// Driver ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DriverId(pub u32);

/// Next device ID counter
static NEXT_DEVICE_ID: AtomicU64 = AtomicU64::new(1);

/// Generate a new unique device ID
pub fn alloc_device_id() -> DeviceId {
    DeviceId(NEXT_DEVICE_ID.fetch_add(1, Ordering::Relaxed))
}

lazy_static! {
    /// Global device manager
    pub static ref DEVICE_MANAGER: Mutex<DeviceManager> = Mutex::new(DeviceManager::new());
}

/// Device Manager
pub struct DeviceManager {
    /// All devices by ID
    devices: BTreeMap<DeviceId, Device>,
    
    /// Root device ID
    root: Option<DeviceId>,
    
    /// Registered drivers
    drivers: BTreeMap<DriverId, DriverInfo>,
    
    /// Next driver ID
    next_driver_id: u32,
}

/// Driver information
pub struct DriverInfo {
    /// Driver name
    pub name: String,
    
    /// Supported device classes
    pub supported_classes: Vec<DeviceClass>,
    
    /// Supported bus types
    pub supported_buses: Vec<BusType>,
}

impl DeviceManager {
    /// Create a new device manager
    pub fn new() -> Self {
        let mut dm = DeviceManager {
            devices: BTreeMap::new(),
            root: None,
            drivers: BTreeMap::new(),
            next_driver_id: 1,
        };
        
        // Create root device
        let root_id = alloc_device_id();
        let root = Device::new(
            root_id,
            "root",
            DeviceClass::Root,
            BusType::Root,
            None,
        );
        dm.devices.insert(root_id, root);
        dm.root = Some(root_id);
        
        dm
    }
    
    /// Get root device ID
    pub fn root(&self) -> Option<DeviceId> {
        self.root
    }
    
    /// Register a new device
    pub fn register_device(&mut self, mut device: Device) -> DeviceId {
        let id = device.id;
        
        // Set parent to root if not specified
        if device.parent.is_none() {
            device.parent = self.root;
        }
        
        // Add to parent's children list
        if let Some(parent_id) = device.parent {
            if let Some(parent) = self.devices.get_mut(&parent_id) {
                parent.children.push(id);
            }
        }
        
        self.devices.insert(id, device);
        id
    }
    
    /// Unregister a device
    pub fn unregister_device(&mut self, id: DeviceId) -> Option<Device> {
        if let Some(device) = self.devices.remove(&id) {
            // Remove from parent's children list
            if let Some(parent_id) = device.parent {
                if let Some(parent) = self.devices.get_mut(&parent_id) {
                    parent.children.retain(|&child_id| child_id != id);
                }
            }
            
            // Recursively remove children
            for child_id in &device.children {
                self.unregister_device(*child_id);
            }
            
            Some(device)
        } else {
            None
        }
    }
    
    /// Get a device by ID
    pub fn get_device(&self, id: DeviceId) -> Option<&Device> {
        self.devices.get(&id)
    }
    
    /// Get a mutable device by ID
    pub fn get_device_mut(&mut self, id: DeviceId) -> Option<&mut Device> {
        self.devices.get_mut(&id)
    }
    
    /// Find devices by class
    pub fn find_by_class(&self, class: DeviceClass) -> Vec<&Device> {
        self.devices.values()
            .filter(|d| d.class == class)
            .collect()
    }
    
    /// Find devices by bus
    pub fn find_by_bus(&self, bus: BusType) -> Vec<&Device> {
        self.devices.values()
            .filter(|d| d.bus == bus)
            .collect()
    }
    
    /// Get all devices
    pub fn all_devices(&self) -> impl Iterator<Item = &Device> {
        self.devices.values()
    }
    
    /// Get device count
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
    
    /// Register a driver
    pub fn register_driver(&mut self, info: DriverInfo) -> DriverId {
        let id = DriverId(self.next_driver_id);
        self.next_driver_id += 1;
        self.drivers.insert(id, info);
        id
    }
    
    /// Bind a driver to a device
    pub fn bind_driver(&mut self, device_id: DeviceId, driver_id: DriverId) -> Result<(), &'static str> {
        let device = self.devices.get_mut(&device_id).ok_or("Device not found")?;
        
        if device.driver.is_some() {
            return Err("Device already has a driver");
        }
        
        if !self.drivers.contains_key(&driver_id) {
            return Err("Driver not found");
        }
        
        device.driver = Some(driver_id);
        device.state = DeviceState::Bound;
        
        Ok(())
    }
    
    /// Unbind driver from device
    pub fn unbind_driver(&mut self, device_id: DeviceId) -> Result<(), &'static str> {
        let device = self.devices.get_mut(&device_id).ok_or("Device not found")?;
        device.driver = None;
        device.state = DeviceState::Registered;
        Ok(())
    }
    
    /// Print device tree
    pub fn print_tree(&self) {
        if let Some(root_id) = self.root {
            self.print_device_tree(root_id, "", true);
        }
    }
    
    fn print_device_tree(&self, id: DeviceId, prefix: &str, is_last: bool) {
        if let Some(device) = self.devices.get(&id) {
            let connector = if is_last { "└── " } else { "├── " };
            crate::println!("{}{}{} [{}:{}] {:?}", 
                prefix, connector, device.name, 
                device.bus.name(), device.class.name(),
                device.state
            );
            
            let child_prefix = if is_last {
                alloc::format!("{}    ", prefix)
            } else {
                alloc::format!("{}│   ", prefix)
            };
            
            let children = &device.children;
            for (i, &child_id) in children.iter().enumerate() {
                let is_last_child = i == children.len() - 1;
                self.print_device_tree(child_id, &child_prefix, is_last_child);
            }
        }
    }
}

/// Initialize the device manager
pub fn init() {
    let mut dm = DEVICE_MANAGER.lock();
    
    // The root device is created in DeviceManager::new()
    crate::println!("  [OK] Device manager initialized");
}

/// Register a platform device
pub fn register_platform_device(name: &str, class: DeviceClass) -> DeviceId {
    let id = alloc_device_id();
    let device = Device::new(id, name, class, BusType::Platform, None);
    
    let mut dm = DEVICE_MANAGER.lock();
    dm.register_device(device)
}

/// Register a VirtIO device
pub fn register_virtio_device(name: &str, class: DeviceClass) -> DeviceId {
    let id = alloc_device_id();
    let device = Device::new(id, name, class, BusType::VirtIO, None);
    
    let mut dm = DEVICE_MANAGER.lock();
    dm.register_device(device)
}

