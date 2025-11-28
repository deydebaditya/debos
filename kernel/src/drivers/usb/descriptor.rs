//! USB Descriptors
//!
//! USB device, configuration, interface, and endpoint descriptors.

/// USB descriptor types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorType {
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
    DeviceQualifier = 6,
    OtherSpeedConfig = 7,
    InterfacePower = 8,
    Otg = 9,
    Debug = 10,
    InterfaceAssociation = 11,
    Bos = 15,
    DeviceCapability = 16,
    SuperSpeedEndpointCompanion = 48,
    SuperSpeedPlusIsochEndpointCompanion = 49,
}

/// USB Device Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DeviceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub usb_version: u16,
    pub device_class: u8,
    pub device_subclass: u8,
    pub device_protocol: u8,
    pub max_packet_size0: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub device_version: u16,
    pub manufacturer_idx: u8,
    pub product_idx: u8,
    pub serial_number_idx: u8,
    pub num_configurations: u8,
}

impl DeviceDescriptor {
    pub const SIZE: u8 = 18;
}

/// USB Configuration Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ConfigurationDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub total_length: u16,
    pub num_interfaces: u8,
    pub configuration_value: u8,
    pub configuration_idx: u8,
    pub attributes: u8,
    pub max_power: u8,
}

impl ConfigurationDescriptor {
    pub const SIZE: u8 = 9;
    
    /// Check if bus-powered
    pub fn is_bus_powered(&self) -> bool {
        (self.attributes & 0x80) != 0
    }
    
    /// Check if self-powered
    pub fn is_self_powered(&self) -> bool {
        (self.attributes & 0x40) != 0
    }
    
    /// Check if supports remote wakeup
    pub fn supports_remote_wakeup(&self) -> bool {
        (self.attributes & 0x20) != 0
    }
    
    /// Get max power in mA
    pub fn max_power_ma(&self) -> u16 {
        self.max_power as u16 * 2
    }
}

/// USB Interface Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct InterfaceDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub interface_number: u8,
    pub alternate_setting: u8,
    pub num_endpoints: u8,
    pub interface_class: u8,
    pub interface_subclass: u8,
    pub interface_protocol: u8,
    pub interface_idx: u8,
}

impl InterfaceDescriptor {
    pub const SIZE: u8 = 9;
}

/// USB Endpoint Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct EndpointDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub endpoint_address: u8,
    pub attributes: u8,
    pub max_packet_size: u16,
    pub interval: u8,
}

impl EndpointDescriptor {
    pub const SIZE: u8 = 7;
    
    /// Get endpoint number
    pub fn number(&self) -> u8 {
        self.endpoint_address & 0x0F
    }
    
    /// Check if IN endpoint
    pub fn is_in(&self) -> bool {
        (self.endpoint_address & 0x80) != 0
    }
    
    /// Check if OUT endpoint
    pub fn is_out(&self) -> bool {
        !self.is_in()
    }
    
    /// Get transfer type
    pub fn transfer_type(&self) -> EndpointTransferType {
        match self.attributes & 0x03 {
            0 => EndpointTransferType::Control,
            1 => EndpointTransferType::Isochronous,
            2 => EndpointTransferType::Bulk,
            3 => EndpointTransferType::Interrupt,
            _ => unreachable!(),
        }
    }
}

/// Endpoint transfer types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointTransferType {
    Control,
    Isochronous,
    Bulk,
    Interrupt,
}

/// USB String Descriptor (first 2 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct StringDescriptorHeader {
    pub length: u8,
    pub descriptor_type: u8,
}

/// HID Descriptor
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct HidDescriptor {
    pub length: u8,
    pub descriptor_type: u8,
    pub hid_version: u16,
    pub country_code: u8,
    pub num_descriptors: u8,
    pub report_descriptor_type: u8,
    pub report_descriptor_length: u16,
}

impl HidDescriptor {
    pub const SIZE: u8 = 9;
}

