//! # DeK - DebOS Nano-Kernel
//!
//! The microkernel core of DebOS. This kernel provides only the bare minimum
//! mechanisms required for a secure, efficient operating system:
//!
//! - **Interrupt Handling**: CPU exceptions and hardware interrupts
//! - **Scheduling**: Preemptive priority-based thread scheduling
//! - **IPC**: Fast inter-process communication primitives
//! - **Memory Management**: Page table management and memory allocation
//!
//! All other OS functionality runs in userspace servers.
//!
//! ## Supported Architectures
//! - x86_64 (Intel/AMD)
//! - AArch64 (ARM64/Apple Silicon)

#![no_std]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]

extern crate alloc;

pub mod arch;
pub mod memory;
pub mod scheduler;
pub mod ipc;
pub mod syscall;
pub mod capability;
pub mod fs;
pub mod shell;

// ============================================================================
// Common kernel initialization
// ============================================================================

/// Common kernel initialization (architecture-independent)
#[no_mangle]
pub fn kernel_init() -> ! {
    println!("[..] Initializing scheduler...");
    scheduler::init();
    println!("[OK] Scheduler initialized");
    
    println!("[..] Initializing filesystem...");
    fs::init();
    println!("[OK] Filesystem initialized");
    
    // Enable interrupts
    println!("[..] Enabling interrupts...");
    #[cfg(target_arch = "x86_64")]
    x86_64::instructions::interrupts::enable();
    #[cfg(target_arch = "aarch64")]
    {
        arch::aarch64::gic::enable_timer();
        arch::enable_interrupts();
    }
    println!("[OK] Interrupts enabled");
    
    println!();
    println!("[OK] Kernel initialization complete");
    println!();
    
    // Start the shell as the main user interaction point
    start_shell();
    
    // Enter idle loop
    idle_loop()
}

/// Start the interactive shell
fn start_shell() {
    // Spawn the shell as a high-priority kernel thread
    let shell_tid = scheduler::spawn_thread(
        shell::shell_thread_entry as *const () as usize, 
        64  // High priority
    );
    println!("[INIT] Shell started with TID: {}", shell_tid);
}

/// Idle loop - puts CPU to sleep waiting for interrupts
pub fn idle_loop() -> ! {
    loop {
        #[cfg(target_arch = "x86_64")]
        x86_64::instructions::hlt();
        
        #[cfg(target_arch = "aarch64")]
        arch::wait_for_interrupt();
    }
}

// ============================================================================
// Print macros (architecture-aware)
// ============================================================================

/// Print to the serial console
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        #[cfg(target_arch = "x86_64")]
        $crate::arch::x86_64::serial::_print(format_args!($($arg)*));
        #[cfg(target_arch = "aarch64")]
        $crate::arch::aarch64::uart::_print(format_args!($($arg)*));
    }};
}

/// Print to the serial console with a newline
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
