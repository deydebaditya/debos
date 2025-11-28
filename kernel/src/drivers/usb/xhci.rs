//! xHCI (USB 3.x) Host Controller Driver
//!
//! Implements the eXtensible Host Controller Interface for USB 3.x support.
//!
//! ## Architecture
//! - MMIO-based register access
//! - Command Ring for host-to-controller commands
//! - Event Ring for controller-to-host events
//! - Transfer Rings for data transfer per endpoint
//! - Device Context Base Address Array (DCBAA)
//!
//! ## References
//! - xHCI Specification 1.2

use alloc::vec::Vec;
use alloc::boxed::Box;
use core::ptr::{read_volatile, write_volatile};
use spin::Mutex;
use lazy_static::lazy_static;

use super::{UsbDevice, UsbSpeed, UsbClass};
use crate::drivers::bus::pci;

/// xHCI Capability Register offsets
mod cap_regs {
    pub const CAPLENGTH: usize = 0x00;      // Capability Length
    pub const HCIVERSION: usize = 0x02;     // Interface Version
    pub const HCSPARAMS1: usize = 0x04;     // Structural Parameters 1
    pub const HCSPARAMS2: usize = 0x08;     // Structural Parameters 2
    pub const HCSPARAMS3: usize = 0x0C;     // Structural Parameters 3
    pub const HCCPARAMS1: usize = 0x10;     // Capability Parameters 1
    pub const DBOFF: usize = 0x14;          // Doorbell Offset
    pub const RTSOFF: usize = 0x18;         // Runtime Register Space Offset
}

/// xHCI Operational Register offsets (relative to op_base)
mod op_regs {
    pub const USBCMD: usize = 0x00;         // USB Command
    pub const USBSTS: usize = 0x04;         // USB Status
    pub const PAGESIZE: usize = 0x08;       // Page Size
    pub const DNCTRL: usize = 0x14;         // Device Notification Control
    pub const CRCR: usize = 0x18;           // Command Ring Control
    pub const DCBAAP: usize = 0x30;         // Device Context Base Address Array Pointer
    pub const CONFIG: usize = 0x38;         // Configure
}

/// xHCI Port Register offsets (per port, relative to port_base)
mod port_regs {
    pub const PORTSC: usize = 0x00;         // Port Status and Control
    pub const PORTPMSC: usize = 0x04;       // Port Power Management Status and Control
    pub const PORTLI: usize = 0x08;         // Port Link Info
    pub const PORTHLPMC: usize = 0x0C;      // Port Hardware LPM Control
}

/// USBCMD bits
mod usbcmd {
    pub const RUN: u32 = 1 << 0;            // Run/Stop
    pub const HCRST: u32 = 1 << 1;          // Host Controller Reset
    pub const INTE: u32 = 1 << 2;           // Interrupter Enable
    pub const HSEE: u32 = 1 << 3;           // Host System Error Enable
}

/// USBSTS bits
mod usbsts {
    pub const HCH: u32 = 1 << 0;            // Host Controller Halted
    pub const HSE: u32 = 1 << 2;            // Host System Error
    pub const EINT: u32 = 1 << 3;           // Event Interrupt
    pub const PCD: u32 = 1 << 4;            // Port Change Detect
    pub const CNR: u32 = 1 << 11;           // Controller Not Ready
}

/// Port speed values
mod port_speed {
    pub const FULL: u32 = 1;
    pub const LOW: u32 = 2;
    pub const HIGH: u32 = 3;
    pub const SUPER: u32 = 4;
    pub const SUPER_PLUS: u32 = 5;
}

/// xHCI Controller
pub struct XhciController {
    /// Base MMIO address
    base: usize,
    
    /// Capability register base
    cap_base: usize,
    
    /// Operational register base
    op_base: usize,
    
    /// Runtime register base
    rt_base: usize,
    
    /// Doorbell register base
    db_base: usize,
    
    /// Port register base
    port_base: usize,
    
    /// Number of ports
    num_ports: u8,
    
    /// Maximum device slots
    max_slots: u8,
    
    /// Maximum interrupters
    max_intrs: u16,
    
    /// Page size (in bytes)
    page_size: u32,
    
    /// Device Context Base Address Array
    dcbaa: Option<Box<[u64; 256]>>,
    
    /// Command Ring
    command_ring: Option<CommandRing>,
    
    /// Is controller running
    running: bool,
}

/// Command Ring
struct CommandRing {
    /// Ring buffer
    buffer: Box<[TRB; 256]>,
    
    /// Current enqueue pointer
    enqueue: usize,
    
    /// Cycle bit
    cycle: bool,
}

/// Transfer Request Block
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TRB {
    pub param1: u64,
    pub param2: u32,
    pub control: u32,
}

impl TRB {
    pub fn new() -> Self {
        TRB::default()
    }
}

impl XhciController {
    /// Create a new xHCI controller
    pub fn new(base: usize) -> Result<Self, &'static str> {
        let cap_base = base;
        
        // Read capability length
        let cap_length = unsafe { read_volatile((cap_base + cap_regs::CAPLENGTH) as *const u8) };
        
        // Read HCI version
        let version = unsafe { read_volatile((cap_base + cap_regs::HCIVERSION) as *const u16) };
        
        if version < 0x0100 {
            return Err("xHCI version too old");
        }
        
        // Read structural parameters
        let hcsparams1 = unsafe { read_volatile((cap_base + cap_regs::HCSPARAMS1) as *const u32) };
        let max_slots = (hcsparams1 & 0xFF) as u8;
        let max_intrs = ((hcsparams1 >> 8) & 0x7FF) as u16;
        let num_ports = ((hcsparams1 >> 24) & 0xFF) as u8;
        
        // Calculate register bases
        let op_base = cap_base + cap_length as usize;
        
        let dboff = unsafe { read_volatile((cap_base + cap_regs::DBOFF) as *const u32) };
        let db_base = cap_base + (dboff & !0x3) as usize;
        
        let rtsoff = unsafe { read_volatile((cap_base + cap_regs::RTSOFF) as *const u32) };
        let rt_base = cap_base + (rtsoff & !0x1F) as usize;
        
        // Port registers start at op_base + 0x400
        let port_base = op_base + 0x400;
        
        // Read page size
        let page_size_reg = unsafe { read_volatile((op_base + op_regs::PAGESIZE) as *const u32) };
        let page_size = 1u32 << (12 + (page_size_reg & 0xFFFF).trailing_zeros());
        
        Ok(XhciController {
            base,
            cap_base,
            op_base,
            rt_base,
            db_base,
            port_base,
            num_ports,
            max_slots,
            max_intrs,
            page_size,
            dcbaa: None,
            command_ring: None,
            running: false,
        })
    }
    
    /// Initialize the controller
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Stop the controller
        self.stop()?;
        
        // Reset the controller
        self.reset()?;
        
        // Allocate DCBAA
        let dcbaa = Box::new([0u64; 256]);
        let dcbaa_addr = dcbaa.as_ptr() as u64;
        
        unsafe {
            write_volatile((self.op_base + op_regs::DCBAAP) as *mut u64, dcbaa_addr);
        }
        
        self.dcbaa = Some(dcbaa);
        
        // Set max device slots
        let config = self.max_slots as u32;
        unsafe {
            write_volatile((self.op_base + op_regs::CONFIG) as *mut u32, config);
        }
        
        // Allocate and set up command ring
        let mut command_ring = CommandRing {
            buffer: Box::new([TRB::default(); 256]),
            enqueue: 0,
            cycle: true,
        };
        
        // Set up link TRB at end of ring
        command_ring.buffer[255].control = (6 << 10) | 1; // Link TRB with cycle bit
        command_ring.buffer[255].param1 = command_ring.buffer.as_ptr() as u64;
        
        let crcr = (command_ring.buffer.as_ptr() as u64) | 1; // RCS = 1
        unsafe {
            write_volatile((self.op_base + op_regs::CRCR) as *mut u64, crcr);
        }
        
        self.command_ring = Some(command_ring);
        
        // Start the controller
        self.start()?;
        
        self.running = true;
        Ok(())
    }
    
    /// Stop the controller
    fn stop(&mut self) -> Result<(), &'static str> {
        let usbcmd = unsafe { read_volatile((self.op_base + op_regs::USBCMD) as *const u32) };
        unsafe {
            write_volatile((self.op_base + op_regs::USBCMD) as *mut u32, usbcmd & !usbcmd::RUN);
        }
        
        // Wait for HCH bit to be set
        for _ in 0..1000 {
            let usbsts = unsafe { read_volatile((self.op_base + op_regs::USBSTS) as *const u32) };
            if usbsts & usbsts::HCH != 0 {
                return Ok(());
            }
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }
        
        Err("Timeout waiting for controller to halt")
    }
    
    /// Reset the controller
    fn reset(&mut self) -> Result<(), &'static str> {
        unsafe {
            write_volatile((self.op_base + op_regs::USBCMD) as *mut u32, usbcmd::HCRST);
        }
        
        // Wait for reset to complete
        for _ in 0..1000 {
            let usbcmd = unsafe { read_volatile((self.op_base + op_regs::USBCMD) as *const u32) };
            if usbcmd & usbcmd::HCRST == 0 {
                // Also wait for CNR to clear
                let usbsts = unsafe { read_volatile((self.op_base + op_regs::USBSTS) as *const u32) };
                if usbsts & usbsts::CNR == 0 {
                    return Ok(());
                }
            }
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }
        
        Err("Timeout waiting for controller reset")
    }
    
    /// Start the controller
    fn start(&mut self) -> Result<(), &'static str> {
        let usbcmd = unsafe { read_volatile((self.op_base + op_regs::USBCMD) as *const u32) };
        unsafe {
            write_volatile((self.op_base + op_regs::USBCMD) as *mut u32, usbcmd | usbcmd::RUN | usbcmd::INTE);
        }
        
        // Wait for HCH bit to clear
        for _ in 0..1000 {
            let usbsts = unsafe { read_volatile((self.op_base + op_regs::USBSTS) as *const u32) };
            if usbsts & usbsts::HCH == 0 {
                return Ok(());
            }
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }
        
        Err("Timeout waiting for controller to start")
    }
    
    /// Get port status
    pub fn port_status(&self, port: u8) -> u32 {
        if port >= self.num_ports {
            return 0;
        }
        
        let offset = port as usize * 0x10;
        unsafe { read_volatile((self.port_base + offset + port_regs::PORTSC) as *const u32) }
    }
    
    /// Check if a device is connected on a port
    pub fn is_port_connected(&self, port: u8) -> bool {
        (self.port_status(port) & 1) != 0
    }
    
    /// Get port speed
    pub fn port_speed(&self, port: u8) -> Option<UsbSpeed> {
        let portsc = self.port_status(port);
        if (portsc & 1) == 0 {
            return None;
        }
        
        let speed = (portsc >> 10) & 0xF;
        Some(match speed {
            port_speed::LOW => UsbSpeed::Low,
            port_speed::FULL => UsbSpeed::Full,
            port_speed::HIGH => UsbSpeed::High,
            port_speed::SUPER => UsbSpeed::Super,
            port_speed::SUPER_PLUS => UsbSpeed::SuperPlus,
            _ => UsbSpeed::Full,
        })
    }
    
    /// Enumerate connected devices
    pub fn enumerate_devices(&self) -> Vec<UsbDevice> {
        let mut devices = Vec::new();
        
        for port in 0..self.num_ports {
            if self.is_port_connected(port) {
                if let Some(speed) = self.port_speed(port) {
                    let mut device = UsbDevice::new((port + 1), speed);
                    device.port = port;
                    devices.push(device);
                }
            }
        }
        
        devices
    }
    
    /// Get number of ports
    pub fn num_ports(&self) -> u8 {
        self.num_ports
    }
    
    /// Get max slots
    pub fn max_slots(&self) -> u8 {
        self.max_slots
    }
    
    /// Check if controller is running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

lazy_static! {
    /// xHCI controllers
    static ref XHCI_CONTROLLERS: Mutex<Vec<XhciController>> = Mutex::new(Vec::new());
}

/// Initialize all xHCI controllers
pub fn init() {
    // Find xHCI controllers on PCI bus
    let controllers = pci::find_xhci_controllers();
    
    if controllers.is_empty() {
        crate::println!("    No xHCI controllers found");
        crate::println!("    (USB support requires xHCI-capable hardware)");
        return;
    }
    
    let mut xhci_list = XHCI_CONTROLLERS.lock();
    
    for pci_dev in controllers {
        if let Some(bar0) = pci_dev.bar_address(0) {
            match XhciController::new(bar0 as usize) {
                Ok(mut controller) => {
                    crate::println!("    xHCI: {} ports, {} slots at {:#x}", 
                        controller.num_ports(), 
                        controller.max_slots(),
                        bar0);
                    
                    if let Err(e) = controller.init() {
                        crate::println!("    Failed to initialize: {}", e);
                    } else {
                        // Enumerate devices
                        let devices = controller.enumerate_devices();
                        for device in devices {
                            crate::println!("      Port {}: {} device", 
                                device.port, 
                                device.speed.name());
                            super::register_device(device);
                        }
                        xhci_list.push(controller);
                    }
                }
                Err(e) => {
                    crate::println!("    Failed to create xHCI: {}", e);
                }
            }
        }
    }
}

/// Check if xHCI is available
pub fn is_available() -> bool {
    !XHCI_CONTROLLERS.lock().is_empty()
}

/// Get total number of USB ports
pub fn total_ports() -> u8 {
    XHCI_CONTROLLERS.lock()
        .iter()
        .map(|c| c.num_ports())
        .sum()
}

