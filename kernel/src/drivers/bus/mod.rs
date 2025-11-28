//! Bus Drivers
//!
//! Provides bus enumeration and management for PCI, USB, and other buses.

pub mod pci;

/// Initialize bus subsystem
pub fn init() {
    crate::println!("  [..] Initializing bus subsystem...");
    
    // Initialize PCI on x86_64
    #[cfg(target_arch = "x86_64")]
    pci::init();
    
    // On AArch64, use device tree or ACPI
    #[cfg(target_arch = "aarch64")]
    {
        // QEMU virt machine uses VirtIO-MMIO, not PCI
        crate::println!("  Note: AArch64 uses VirtIO-MMIO (not PCI)");
    }
    
    crate::println!("  [OK] Bus subsystem initialized");
}

