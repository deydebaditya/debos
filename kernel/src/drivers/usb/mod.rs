//! USB Subsystem
//!
//! Provides USB host controller and device support.

pub mod xhci;
pub mod descriptor;
pub mod device;

use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;

/// USB device speed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbSpeed {
    Low,      // 1.5 Mbps (USB 1.0)
    Full,     // 12 Mbps (USB 1.1)
    High,     // 480 Mbps (USB 2.0)
    Super,    // 5 Gbps (USB 3.0)
    SuperPlus, // 10 Gbps (USB 3.1)
}

impl UsbSpeed {
    pub fn name(&self) -> &'static str {
        match self {
            UsbSpeed::Low => "Low (1.5 Mbps)",
            UsbSpeed::Full => "Full (12 Mbps)",
            UsbSpeed::High => "High (480 Mbps)",
            UsbSpeed::Super => "Super (5 Gbps)",
            UsbSpeed::SuperPlus => "Super+ (10 Gbps)",
        }
    }
}

/// USB device class codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UsbClass {
    PerInterface = 0x00,
    Audio = 0x01,
    Communications = 0x02,
    Hid = 0x03,
    Physical = 0x05,
    Image = 0x06,
    Printer = 0x07,
    MassStorage = 0x08,
    Hub = 0x09,
    CdcData = 0x0A,
    SmartCard = 0x0B,
    ContentSecurity = 0x0D,
    Video = 0x0E,
    PersonalHealthcare = 0x0F,
    AudioVideo = 0x10,
    Billboard = 0x11,
    UsbTypeCBridge = 0x12,
    Diagnostic = 0xDC,
    Wireless = 0xE0,
    Miscellaneous = 0xEF,
    ApplicationSpecific = 0xFE,
    VendorSpecific = 0xFF,
}

impl UsbClass {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0x00 => UsbClass::PerInterface,
            0x01 => UsbClass::Audio,
            0x02 => UsbClass::Communications,
            0x03 => UsbClass::Hid,
            0x05 => UsbClass::Physical,
            0x06 => UsbClass::Image,
            0x07 => UsbClass::Printer,
            0x08 => UsbClass::MassStorage,
            0x09 => UsbClass::Hub,
            0x0A => UsbClass::CdcData,
            0x0B => UsbClass::SmartCard,
            0x0D => UsbClass::ContentSecurity,
            0x0E => UsbClass::Video,
            0x0F => UsbClass::PersonalHealthcare,
            0x10 => UsbClass::AudioVideo,
            0x11 => UsbClass::Billboard,
            0x12 => UsbClass::UsbTypeCBridge,
            0xDC => UsbClass::Diagnostic,
            0xE0 => UsbClass::Wireless,
            0xEF => UsbClass::Miscellaneous,
            0xFE => UsbClass::ApplicationSpecific,
            _ => UsbClass::VendorSpecific,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            UsbClass::PerInterface => "Per Interface",
            UsbClass::Audio => "Audio",
            UsbClass::Communications => "Communications",
            UsbClass::Hid => "HID",
            UsbClass::Physical => "Physical",
            UsbClass::Image => "Image",
            UsbClass::Printer => "Printer",
            UsbClass::MassStorage => "Mass Storage",
            UsbClass::Hub => "Hub",
            UsbClass::CdcData => "CDC Data",
            UsbClass::SmartCard => "Smart Card",
            UsbClass::ContentSecurity => "Content Security",
            UsbClass::Video => "Video",
            UsbClass::PersonalHealthcare => "Personal Healthcare",
            UsbClass::AudioVideo => "Audio/Video",
            UsbClass::Billboard => "Billboard",
            UsbClass::UsbTypeCBridge => "USB Type-C Bridge",
            UsbClass::Diagnostic => "Diagnostic",
            UsbClass::Wireless => "Wireless",
            UsbClass::Miscellaneous => "Miscellaneous",
            UsbClass::ApplicationSpecific => "Application Specific",
            UsbClass::VendorSpecific => "Vendor Specific",
        }
    }
}

/// USB device information
#[derive(Debug, Clone)]
pub struct UsbDevice {
    /// Device address (1-127)
    pub address: u8,
    
    /// Speed
    pub speed: UsbSpeed,
    
    /// Vendor ID
    pub vendor_id: u16,
    
    /// Product ID
    pub product_id: u16,
    
    /// Device class
    pub class: UsbClass,
    
    /// Subclass
    pub subclass: u8,
    
    /// Protocol
    pub protocol: u8,
    
    /// Manufacturer string
    pub manufacturer: Option<String>,
    
    /// Product string
    pub product: Option<String>,
    
    /// Serial number string
    pub serial: Option<String>,
    
    /// Port on parent hub (0 for root)
    pub port: u8,
    
    /// Parent hub address (0 for root)
    pub parent: u8,
}

impl UsbDevice {
    pub fn new(address: u8, speed: UsbSpeed) -> Self {
        UsbDevice {
            address,
            speed,
            vendor_id: 0,
            product_id: 0,
            class: UsbClass::PerInterface,
            subclass: 0,
            protocol: 0,
            manufacturer: None,
            product: None,
            serial: None,
            port: 0,
            parent: 0,
        }
    }
}

lazy_static! {
    /// Enumerated USB devices
    static ref USB_DEVICES: Mutex<Vec<UsbDevice>> = Mutex::new(Vec::new());
}

/// Initialize USB subsystem
pub fn init() {
    crate::println!("  [..] Initializing USB subsystem...");
    
    // Initialize xHCI controllers
    xhci::init();
    
    crate::println!("  [OK] USB subsystem initialized");
}

/// Get all USB devices
pub fn get_devices() -> Vec<UsbDevice> {
    USB_DEVICES.lock().clone()
}

/// Register a USB device
pub fn register_device(device: UsbDevice) {
    USB_DEVICES.lock().push(device);
}

/// Find USB device by vendor and product ID
pub fn find_device(vendor_id: u16, product_id: u16) -> Option<UsbDevice> {
    USB_DEVICES.lock()
        .iter()
        .find(|d| d.vendor_id == vendor_id && d.product_id == product_id)
        .cloned()
}

/// Find USB devices by class
pub fn find_by_class(class: UsbClass) -> Vec<UsbDevice> {
    USB_DEVICES.lock()
        .iter()
        .filter(|d| d.class == class)
        .cloned()
        .collect()
}

