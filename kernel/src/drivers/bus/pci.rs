//! PCI/PCIe Bus Driver
//!
//! Provides PCI configuration space access and device enumeration.
//!
//! ## PCI Configuration Space
//! - Uses ECAM (Enhanced Configuration Access Mechanism) for PCIe
//! - Falls back to I/O ports 0xCF8/0xCFC for legacy PCI
//!
//! ## Addressing
//! - Bus: 0-255
//! - Device: 0-31
//! - Function: 0-7
//!
//! ## Configuration Space Layout (first 64 bytes)
//! - 0x00: Vendor ID (16-bit)
//! - 0x02: Device ID (16-bit)
//! - 0x04: Command (16-bit)
//! - 0x06: Status (16-bit)
//! - 0x08: Revision ID (8-bit)
//! - 0x09: Prog IF (8-bit)
//! - 0x0A: Subclass (8-bit)
//! - 0x0B: Class Code (8-bit)
//! - 0x10-0x27: BAR0-BAR5 (32-bit each)
//! - 0x2C: Subsystem Vendor ID (16-bit)
//! - 0x2E: Subsystem ID (16-bit)
//! - 0x3C: Interrupt Line (8-bit)
//! - 0x3D: Interrupt Pin (8-bit)

use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;

use crate::drivers::device::{Device, DeviceClass, BusType, alloc_device_id, DEVICE_MANAGER};

/// PCI configuration address port (x86 only)
#[cfg(target_arch = "x86_64")]
const PCI_CONFIG_ADDRESS: u16 = 0xCF8;

/// PCI configuration data port (x86 only)
#[cfg(target_arch = "x86_64")]
const PCI_CONFIG_DATA: u16 = 0xCFC;

/// Invalid vendor ID (indicates no device)
const PCI_VENDOR_INVALID: u16 = 0xFFFF;

/// PCI device class codes
pub mod ClassCode {
    pub const UNCLASSIFIED: u8 = 0x00;
    pub const MASS_STORAGE: u8 = 0x01;
    pub const NETWORK: u8 = 0x02;
    pub const DISPLAY: u8 = 0x03;
    pub const MULTIMEDIA: u8 = 0x04;
    pub const MEMORY: u8 = 0x05;
    pub const BRIDGE: u8 = 0x06;
    pub const SIMPLE_COMM: u8 = 0x07;
    pub const BASE_PERIPHERAL: u8 = 0x08;
    pub const INPUT: u8 = 0x09;
    pub const DOCKING: u8 = 0x0A;
    pub const PROCESSOR: u8 = 0x0B;
    pub const SERIAL_BUS: u8 = 0x0C;
    pub const WIRELESS: u8 = 0x0D;
    pub const INTELLIGENT_IO: u8 = 0x0E;
    pub const SATELLITE: u8 = 0x0F;
    pub const ENCRYPTION: u8 = 0x10;
    pub const SIGNAL_PROCESSING: u8 = 0x11;
}

/// PCI subclass codes for Serial Bus
pub mod SerialBusSubclass {
    pub const FIREWIRE: u8 = 0x00;
    pub const ACCESS_BUS: u8 = 0x01;
    pub const SSA: u8 = 0x02;
    pub const USB: u8 = 0x03;
    pub const FIBRE_CHANNEL: u8 = 0x04;
    pub const SMBUS: u8 = 0x05;
    pub const INFINIBAND: u8 = 0x06;
    pub const IPMI: u8 = 0x07;
}

/// USB Controller types (Prog IF)
pub mod UsbProgIf {
    pub const UHCI: u8 = 0x00;
    pub const OHCI: u8 = 0x10;
    pub const EHCI: u8 = 0x20;
    pub const XHCI: u8 = 0x30;
}

/// PCI device information
#[derive(Debug, Clone)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub revision: u8,
    pub header_type: u8,
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
    pub bars: [u32; 6],
}

impl PciDevice {
    /// Get the BDF (Bus:Device.Function) address
    pub fn bdf(&self) -> String {
        alloc::format!("{:02x}:{:02x}.{}", self.bus, self.device, self.function)
    }
    
    /// Get class name
    pub fn class_name(&self) -> &'static str {
        match self.class_code {
            ClassCode::UNCLASSIFIED => "Unclassified",
            ClassCode::MASS_STORAGE => "Mass Storage",
            ClassCode::NETWORK => "Network",
            ClassCode::DISPLAY => "Display",
            ClassCode::MULTIMEDIA => "Multimedia",
            ClassCode::MEMORY => "Memory",
            ClassCode::BRIDGE => "Bridge",
            ClassCode::SIMPLE_COMM => "Communication",
            ClassCode::BASE_PERIPHERAL => "Peripheral",
            ClassCode::INPUT => "Input",
            ClassCode::DOCKING => "Docking",
            ClassCode::PROCESSOR => "Processor",
            ClassCode::SERIAL_BUS => "Serial Bus",
            ClassCode::WIRELESS => "Wireless",
            ClassCode::INTELLIGENT_IO => "Intelligent I/O",
            ClassCode::SATELLITE => "Satellite",
            ClassCode::ENCRYPTION => "Encryption",
            ClassCode::SIGNAL_PROCESSING => "Signal Processing",
            _ => "Unknown",
        }
    }
    
    /// Check if this is a USB controller
    pub fn is_usb_controller(&self) -> bool {
        self.class_code == ClassCode::SERIAL_BUS && 
        self.subclass == SerialBusSubclass::USB
    }
    
    /// Get USB controller type
    pub fn usb_controller_type(&self) -> Option<&'static str> {
        if !self.is_usb_controller() {
            return None;
        }
        
        Some(match self.prog_if {
            UsbProgIf::UHCI => "UHCI (USB 1.x)",
            UsbProgIf::OHCI => "OHCI (USB 1.x)",
            UsbProgIf::EHCI => "EHCI (USB 2.0)",
            UsbProgIf::XHCI => "xHCI (USB 3.x)",
            _ => "Unknown USB",
        })
    }
    
    /// Check if this is a multifunction device
    pub fn is_multifunction(&self) -> bool {
        (self.header_type & 0x80) != 0
    }
    
    /// Get BAR as memory address
    pub fn bar_address(&self, index: usize) -> Option<u64> {
        if index >= 6 {
            return None;
        }
        
        let bar = self.bars[index];
        if bar == 0 {
            return None;
        }
        
        // Check if it's memory-mapped (bit 0 = 0)
        if (bar & 1) == 0 {
            // Check for 64-bit BAR
            if (bar & 0x06) == 0x04 && index < 5 {
                let high = self.bars[index + 1] as u64;
                Some(((high << 32) | (bar & 0xFFFFFFF0) as u64))
            } else {
                Some((bar & 0xFFFFFFF0) as u64)
            }
        } else {
            // I/O port address
            Some((bar & 0xFFFFFFFC) as u64)
        }
    }
}

lazy_static! {
    /// Discovered PCI devices
    static ref PCI_DEVICES: Mutex<Vec<PciDevice>> = Mutex::new(Vec::new());
}

/// Read from PCI configuration space (x86_64)
#[cfg(target_arch = "x86_64")]
pub fn config_read(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    use x86_64::instructions::port::Port;
    
    let address: u32 = ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC)
        | 0x80000000;
    
    unsafe {
        let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDRESS);
        let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);
        
        addr_port.write(address);
        data_port.read()
    }
}

/// Write to PCI configuration space (x86_64)
#[cfg(target_arch = "x86_64")]
pub fn config_write(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    use x86_64::instructions::port::Port;
    
    let address: u32 = ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC)
        | 0x80000000;
    
    unsafe {
        let mut addr_port = Port::<u32>::new(PCI_CONFIG_ADDRESS);
        let mut data_port = Port::<u32>::new(PCI_CONFIG_DATA);
        
        addr_port.write(address);
        data_port.write(value);
    }
}

/// Stub for AArch64 (PCI not available on QEMU virt)
#[cfg(target_arch = "aarch64")]
pub fn config_read(_bus: u8, _device: u8, _function: u8, _offset: u8) -> u32 {
    0xFFFFFFFF
}

#[cfg(target_arch = "aarch64")]
pub fn config_write(_bus: u8, _device: u8, _function: u8, _offset: u8, _value: u32) {
    // No-op on AArch64 QEMU virt
}

/// Read 16-bit value from PCI config space
pub fn config_read16(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let value = config_read(bus, device, function, offset & 0xFC);
    ((value >> ((offset & 2) * 8)) & 0xFFFF) as u16
}

/// Read 8-bit value from PCI config space
pub fn config_read8(bus: u8, device: u8, function: u8, offset: u8) -> u8 {
    let value = config_read(bus, device, function, offset & 0xFC);
    ((value >> ((offset & 3) * 8)) & 0xFF) as u8
}

/// Probe a specific PCI slot
fn probe_device(bus: u8, device: u8, function: u8) -> Option<PciDevice> {
    let vendor_id = config_read16(bus, device, function, 0x00);
    
    if vendor_id == PCI_VENDOR_INVALID {
        return None;
    }
    
    let device_id = config_read16(bus, device, function, 0x02);
    let class_code = config_read8(bus, device, function, 0x0B);
    let subclass = config_read8(bus, device, function, 0x0A);
    let prog_if = config_read8(bus, device, function, 0x09);
    let revision = config_read8(bus, device, function, 0x08);
    let header_type = config_read8(bus, device, function, 0x0E);
    let interrupt_line = config_read8(bus, device, function, 0x3C);
    let interrupt_pin = config_read8(bus, device, function, 0x3D);
    
    // Read BARs
    let mut bars = [0u32; 6];
    for i in 0..6 {
        bars[i] = config_read(bus, device, function, 0x10 + (i as u8) * 4);
    }
    
    Some(PciDevice {
        bus,
        device,
        function,
        vendor_id,
        device_id,
        class_code,
        subclass,
        prog_if,
        revision,
        header_type,
        interrupt_line,
        interrupt_pin,
        bars,
    })
}

/// Enumerate all PCI devices
pub fn enumerate() -> Vec<PciDevice> {
    let mut devices = Vec::new();
    
    // Scan all buses
    for bus in 0..=255u8 {
        for device in 0..32u8 {
            if let Some(dev) = probe_device(bus, device, 0) {
                let is_multifunction = dev.is_multifunction();
                devices.push(dev);
                
                // Check other functions if multifunction
                if is_multifunction {
                    for function in 1..8u8 {
                        if let Some(dev) = probe_device(bus, device, function) {
                            devices.push(dev);
                        }
                    }
                }
            }
        }
    }
    
    devices
}

/// Initialize PCI subsystem
pub fn init() {
    crate::println!("  [..] Scanning PCI bus...");
    
    let devices = enumerate();
    
    if devices.is_empty() {
        crate::println!("  No PCI devices found");
        crate::println!("  (This is normal on QEMU virt which uses VirtIO-MMIO)");
    } else {
        crate::println!("  Found {} PCI devices:", devices.len());
        
        let mut dm = DEVICE_MANAGER.lock();
        
        for dev in &devices {
            crate::println!("    {} [{:04x}:{:04x}] {} - {}", 
                dev.bdf(), 
                dev.vendor_id, 
                dev.device_id,
                dev.class_name(),
                if dev.is_usb_controller() {
                    dev.usb_controller_type().unwrap_or("Unknown")
                } else {
                    ""
                }
            );
            
            // Register in device manager
            let class = match dev.class_code {
                ClassCode::NETWORK => DeviceClass::Ethernet,
                ClassCode::DISPLAY => DeviceClass::DisplayController,
                ClassCode::MASS_STORAGE => DeviceClass::BlockDevice,
                ClassCode::SERIAL_BUS if dev.is_usb_controller() => DeviceClass::UsbController,
                ClassCode::INPUT => DeviceClass::GenericInput,
                _ => DeviceClass::Unknown(dev.class_code as u16),
            };
            
            let id = alloc_device_id();
            let mut device = Device::new_pci(
                id,
                &alloc::format!("pci-{}", dev.bdf()),
                class,
                dev.vendor_id,
                dev.device_id,
                None,
            );
            
            // Add BAR resources
            for i in 0..6 {
                if let Some(addr) = dev.bar_address(i) {
                    device.add_mmio(addr as usize, 0x1000); // Assume 4KB minimum
                }
            }
            
            // Add IRQ
            if dev.interrupt_pin > 0 {
                device.add_irq(dev.interrupt_line as u32);
            }
            
            dm.register_device(device);
        }
    }
    
    *PCI_DEVICES.lock() = devices;
    
    crate::println!("  [OK] PCI initialized");
}

/// Get all discovered PCI devices
pub fn get_devices() -> Vec<PciDevice> {
    PCI_DEVICES.lock().clone()
}

/// Find PCI device by vendor and device ID
pub fn find_device(vendor_id: u16, device_id: u16) -> Option<PciDevice> {
    PCI_DEVICES.lock()
        .iter()
        .find(|d| d.vendor_id == vendor_id && d.device_id == device_id)
        .cloned()
}

/// Find all USB controllers
pub fn find_usb_controllers() -> Vec<PciDevice> {
    PCI_DEVICES.lock()
        .iter()
        .filter(|d| d.is_usb_controller())
        .cloned()
        .collect()
}

/// Find all xHCI (USB 3.x) controllers
pub fn find_xhci_controllers() -> Vec<PciDevice> {
    PCI_DEVICES.lock()
        .iter()
        .filter(|d| d.is_usb_controller() && d.prog_if == UsbProgIf::XHCI)
        .cloned()
        .collect()
}

