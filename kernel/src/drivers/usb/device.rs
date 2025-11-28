//! USB Device Management
//!
//! USB device enumeration and management.

use alloc::vec::Vec;
use alloc::string::String;

use super::descriptor::{DeviceDescriptor, ConfigurationDescriptor, InterfaceDescriptor};
use super::{UsbDevice, UsbSpeed, UsbClass};

/// USB device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// Device is not attached
    Detached,
    /// Device is attached but not addressed
    Attached,
    /// Device is powered
    Powered,
    /// Device has been reset
    Default,
    /// Device has been assigned an address
    Addressed,
    /// Device is configured
    Configured,
    /// Device is suspended
    Suspended,
}

/// USB device information (extended)
#[derive(Debug, Clone)]
pub struct UsbDeviceInfo {
    /// Basic device info
    pub device: UsbDevice,
    
    /// Device state
    pub state: DeviceState,
    
    /// Device descriptor
    pub device_descriptor: Option<DeviceDescriptor>,
    
    /// Active configuration
    pub active_config: u8,
    
    /// Number of interfaces
    pub num_interfaces: u8,
}

impl UsbDeviceInfo {
    pub fn new(device: UsbDevice) -> Self {
        UsbDeviceInfo {
            device,
            state: DeviceState::Attached,
            device_descriptor: None,
            active_config: 0,
            num_interfaces: 0,
        }
    }
    
    /// Get USB version string
    pub fn usb_version(&self) -> Option<String> {
        self.device_descriptor.map(|desc| {
            let major = (desc.usb_version >> 8) & 0xFF;
            let minor = (desc.usb_version >> 4) & 0x0F;
            let patch = desc.usb_version & 0x0F;
            alloc::format!("{}.{}.{}", major, minor, patch)
        })
    }
    
    /// Get device class
    pub fn class(&self) -> UsbClass {
        self.device_descriptor
            .map(|desc| UsbClass::from_u8(desc.device_class))
            .unwrap_or(self.device.class)
    }
}

/// USB Hub
#[derive(Debug, Clone)]
pub struct UsbHub {
    /// Hub device
    pub device: UsbDevice,
    
    /// Number of downstream ports
    pub num_ports: u8,
    
    /// Power-on to power-good time (in 2ms units)
    pub power_on_time: u8,
    
    /// Hub controller current (in mA)
    pub hub_current: u16,
    
    /// Per-port power switching
    pub per_port_power: bool,
    
    /// Over-current protection mode
    pub overcurrent_protection: bool,
    
    /// Compound device
    pub compound: bool,
    
    /// Connected devices on each port
    pub downstream: Vec<Option<UsbDevice>>,
}

impl UsbHub {
    pub fn new(device: UsbDevice, num_ports: u8) -> Self {
        UsbHub {
            device,
            num_ports,
            power_on_time: 100, // 200ms default
            hub_current: 100,
            per_port_power: true,
            overcurrent_protection: true,
            compound: false,
            downstream: (0..num_ports).map(|_| None).collect(),
        }
    }
    
    /// Check if a port has a device connected
    pub fn is_port_connected(&self, port: u8) -> bool {
        if port >= self.num_ports {
            return false;
        }
        self.downstream[port as usize].is_some()
    }
    
    /// Get device on a port
    pub fn get_port_device(&self, port: u8) -> Option<&UsbDevice> {
        if port >= self.num_ports {
            return None;
        }
        self.downstream[port as usize].as_ref()
    }
}

/// Standard USB requests (bRequest values)
pub mod StandardRequest {
    pub const GET_STATUS: u8 = 0;
    pub const CLEAR_FEATURE: u8 = 1;
    pub const SET_FEATURE: u8 = 3;
    pub const SET_ADDRESS: u8 = 5;
    pub const GET_DESCRIPTOR: u8 = 6;
    pub const SET_DESCRIPTOR: u8 = 7;
    pub const GET_CONFIGURATION: u8 = 8;
    pub const SET_CONFIGURATION: u8 = 9;
    pub const GET_INTERFACE: u8 = 10;
    pub const SET_INTERFACE: u8 = 11;
    pub const SYNCH_FRAME: u8 = 12;
}

/// USB Setup Packet
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct SetupPacket {
    /// Request type
    pub request_type: u8,
    /// Request
    pub request: u8,
    /// Value
    pub value: u16,
    /// Index
    pub index: u16,
    /// Length
    pub length: u16,
}

impl SetupPacket {
    pub const SIZE: usize = 8;
    
    /// Create a GET_DESCRIPTOR setup packet
    pub fn get_descriptor(desc_type: u8, desc_index: u8, length: u16) -> Self {
        SetupPacket {
            request_type: 0x80, // Device-to-host, standard, device
            request: StandardRequest::GET_DESCRIPTOR,
            value: ((desc_type as u16) << 8) | (desc_index as u16),
            index: 0,
            length,
        }
    }
    
    /// Create a SET_ADDRESS setup packet
    pub fn set_address(address: u8) -> Self {
        SetupPacket {
            request_type: 0x00, // Host-to-device, standard, device
            request: StandardRequest::SET_ADDRESS,
            value: address as u16,
            index: 0,
            length: 0,
        }
    }
    
    /// Create a SET_CONFIGURATION setup packet
    pub fn set_configuration(config: u8) -> Self {
        SetupPacket {
            request_type: 0x00,
            request: StandardRequest::SET_CONFIGURATION,
            value: config as u16,
            index: 0,
            length: 0,
        }
    }
}

