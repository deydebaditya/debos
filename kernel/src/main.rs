//! DebOS Kernel Binary Entry Point
//!
//! This creates a directly bootable ELF for AArch64.

#![no_std]
#![no_main]
#![allow(warnings)] // Suppress all compiler warnings
#![feature(alloc_error_handler)]

extern crate alloc;

// Re-export everything from the kernel library
pub use debos_kernel::*;

// ============================================================================
// AArch64 Boot Code
// ============================================================================

#[cfg(target_arch = "aarch64")]
use core::arch::global_asm;

#[cfg(target_arch = "aarch64")]
global_asm!(
    r#"
.section .text.boot
.global _start

_start:
    // Check processor ID, only boot on core 0
    mrs     x1, mpidr_el1
    and     x1, x1, #3
    cbz     x1, 2f
    
    // Other cores: wait for events (parking)
1:  wfe
    b       1b

2:  // Core 0 continues here
    
    // Enable FPU/SIMD (CPACR_EL1)
    // Set FPEN bits [21:20] to 0b11 to enable FP/SIMD at EL1
    mov     x0, #(3 << 20)
    msr     CPACR_EL1, x0
    isb
    
    // Set up stack pointer using linker-defined symbol
    ldr     x1, =__stack_top
    mov     sp, x1
    
    // Clear BSS section
    ldr     x1, =__bss_start
    ldr     x2, =__bss_end
3:  cmp     x1, x2
    b.ge    4f
    str     xzr, [x1], #8
    b       3b
    
4:  // Jump to Rust entry point
    bl      kernel_main_aarch64
    
    // If kernel_main returns, halt
5:  wfi
    b       5b
"#
);

/// AArch64 kernel entry point
#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub extern "C" fn kernel_main_aarch64() -> ! {
    // Initialize UART first for debugging output
    debos_kernel::arch::aarch64::uart::init();
    
    debos_kernel::println!("╔═══════════════════════════════════════════════════════════════╗");
    debos_kernel::println!("║                DebOS Nano-Kernel (AArch64)                    ║");
    debos_kernel::println!("║                       Version 0.1.0                           ║");
    debos_kernel::println!("╚═══════════════════════════════════════════════════════════════╝");
    debos_kernel::println!();
    
    debos_kernel::println!("[OK] UART initialized");
    
    // Initialize exceptions
    debos_kernel::println!("[..] Initializing exceptions...");
    debos_kernel::arch::aarch64::exceptions::init();
    
    // Initialize GIC
    debos_kernel::println!("[..] Initializing GIC...");
    debos_kernel::arch::aarch64::gic::init();
    
    // Initialize memory
    debos_kernel::println!("[..] Initializing memory...");
    debos_kernel::memory::init_aarch64();
    debos_kernel::println!("[OK] Memory initialized");
    
    // Continue with common kernel init
    debos_kernel::kernel_init()
}

// ============================================================================
// Panic and Allocation Error Handlers
// ============================================================================

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    debos_kernel::println!();
    debos_kernel::println!("╔═══════════════════════════════════════════════════════════════╗");
    debos_kernel::println!("║                     KERNEL PANIC!                             ║");
    debos_kernel::println!("╚═══════════════════════════════════════════════════════════════╝");
    debos_kernel::println!("{}", info);
    
    loop {
        #[cfg(target_arch = "aarch64")]
        unsafe { core::arch::asm!("wfi") };
        #[cfg(target_arch = "x86_64")]
        unsafe { core::arch::asm!("hlt") };
    }
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}
