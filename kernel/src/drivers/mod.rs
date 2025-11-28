//! Device Drivers
//!
//! This module contains in-kernel device drivers for DebOS.
//! In a pure microkernel, these would run in userspace, but for
//! initial development we implement them in the kernel.

pub mod virtio;
pub mod block;

/// Initialize all drivers
pub fn init() {
    crate::println!("[..] Initializing drivers...");
    
    // Initialize VirtIO subsystem
    virtio::init();
    
    crate::println!("[OK] Drivers initialized");
}

