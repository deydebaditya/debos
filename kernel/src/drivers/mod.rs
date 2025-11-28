//! Device Drivers
//!
//! This module contains in-kernel device drivers for DebOS.
//! In a pure microkernel, these would run in userspace, but for
//! initial development we implement them in the kernel.

pub mod virtio;
pub mod block;
pub mod device;
pub mod input;
pub mod net;
pub mod bus;
pub mod usb;

/// Initialize all drivers
pub fn init() {
    crate::println!("[..] Initializing drivers...");
    
    // Initialize Device Manager
    device::init();
    
    // Initialize Bus subsystem (PCI)
    bus::init();
    
    // Initialize VirtIO subsystem
    virtio::init();
    
    // Initialize USB subsystem
    usb::init();
    
    // Initialize Input subsystem
    input::init();
    
    // Initialize Network subsystem
    net::init();
    
    crate::println!("[OK] Drivers initialized");
}

