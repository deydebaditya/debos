//! AArch64 Architecture Support
//!
//! This module provides all AArch64-specific functionality:
//! - Exception handling (Exception Levels EL0-EL3)
//! - MMU (4-level page tables)
//! - GIC (Generic Interrupt Controller)
//! - Context switching
//! - PL011 UART output

pub mod boot;
pub mod exceptions;
pub mod mmu;
pub mod context;
pub mod uart;
pub mod gic;

pub use context::ArchContext;

/// Initialize AArch64-specific hardware
pub fn init() {
    // UART is initialized in boot
    uart::init();
    
    // Set up exception vectors
    exceptions::init();
    
    // Initialize GIC
    gic::init();
}

/// Halt the CPU until an interrupt
#[inline(always)]
pub fn wait_for_interrupt() {
    unsafe {
        core::arch::asm!("wfi");
    }
}

/// Disable interrupts and return previous state
#[inline(always)]
pub fn disable_interrupts() -> bool {
    let daif: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, DAIF",
            "msr DAIFSet, #0xf",
            out(reg) daif
        );
    }
    (daif & 0x80) == 0 // Return true if IRQs were enabled
}

/// Enable interrupts
#[inline(always)]
pub fn enable_interrupts() {
    unsafe {
        core::arch::asm!("msr DAIFClr, #0xf");
    }
}

/// Restore interrupt state
#[inline(always)]
pub fn restore_interrupts(enabled: bool) {
    if enabled {
        enable_interrupts();
    }
}

