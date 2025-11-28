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

/// Initialize all drivers
pub fn init() {
    crate::println!("[..] Initializing drivers...");
    
    // Initialize Device Manager
    device::init();
    
    // Initialize VirtIO subsystem
    virtio::init();
    
    // Initialize Input subsystem
    input::init();
    
    // Initialize Network subsystem
    net::init();
    
    crate::println!("[OK] Drivers initialized");
}

