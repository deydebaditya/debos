//! Shell Commands
//!
//! Built-in commands for the DebOS shell.

use crate::{println, print};
use crate::scheduler;
use crate::memory;

/// Display help information
pub fn help(_args: &[&str]) {
    println!("DebOS Shell Commands:");
    println!("=====================");
    println!();
    println!("  help, ?        - Show this help message");
    println!("  info, sysinfo  - Display system information");
    println!("  mem, memory    - Show memory statistics");
    println!("  threads, ps    - List running threads");
    println!("  uptime         - Show system uptime");
    println!("  echo <text>    - Echo text to the console");
    println!("  clear, cls     - Clear the screen");
    println!("  exit, quit     - Exit the shell");
    println!("  reboot         - Reboot the system (if supported)");
    println!();
}

/// Display system information
pub fn sysinfo(_args: &[&str]) {
    println!("DebOS System Information");
    println!("========================");
    println!();
    
    #[cfg(target_arch = "x86_64")]
    println!("  Architecture:  x86_64");
    
    #[cfg(target_arch = "aarch64")]
    println!("  Architecture:  AArch64 (ARM64)");
    
    println!("  Kernel:        DeK (DebOS Nano-Kernel) v0.1.0");
    println!("  Type:          Microkernel");
    
    // Memory info
    let (heap_used, heap_free) = memory::heap::stats();
    println!("  Heap Used:     {} KB", heap_used / 1024);
    println!("  Heap Free:     {} KB", heap_free / 1024);
    
    // Uptime
    let ticks = scheduler::ticks();
    let seconds = ticks / 100; // Assuming ~100 ticks per second
    println!("  Uptime:        {} seconds ({} ticks)", seconds, ticks);
    
    println!();
}

/// Display memory statistics
pub fn memory(_args: &[&str]) {
    println!("Memory Statistics");
    println!("=================");
    println!();
    
    // Heap stats
    let (heap_used, heap_free) = memory::heap::stats();
    let heap_total = heap_used + heap_free;
    
    // Calculate percentage without floating point
    let used_pct = if heap_total > 0 { (heap_used * 100) / heap_total } else { 0 };
    let free_pct = if heap_total > 0 { (heap_free * 100) / heap_total } else { 0 };
    
    println!("  Heap:");
    println!("    Total:  {} KB", heap_total / 1024);
    println!("    Used:   {} KB ({}%)", heap_used / 1024, used_pct);
    println!("    Free:   {} KB ({}%)", heap_free / 1024, free_pct);
    
    // Physical memory stats
    let (phys_total, phys_free) = memory::buddy::stats();
    if phys_total > 0 {
        println!();
        println!("  Physical Memory (Buddy Allocator):");
        println!("    Total:  {} MB", phys_total / 1024 / 1024);
        println!("    Free:   {} MB", phys_free / 1024 / 1024);
    }
    
    println!();
}

/// List running threads
pub fn threads(_args: &[&str]) {
    println!("Running Threads");
    println!("===============");
    println!();
    println!("  TID   State      Priority   Name");
    println!("  ---   -----      --------   ----");
    
    // Get current thread ID
    if let Some(tid) = scheduler::current_tid() {
        println!("  {}     Running    128        shell", tid);
    }
    
    // TODO: Iterate over all threads when we have thread enumeration
    println!();
    println!("  (Full thread listing not yet implemented)");
    println!();
}

/// Display system uptime
pub fn uptime(_args: &[&str]) {
    let ticks = scheduler::ticks();
    let total_seconds = ticks / 100; // Assuming ~100 ticks per second
    
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    
    println!("System uptime: {}h {}m {}s ({} ticks)", hours, minutes, seconds, ticks);
}

/// Echo text to console
pub fn echo(args: &[&str]) {
    if args.is_empty() {
        println!();
    } else {
        println!("{}", args.join(" "));
    }
}

/// Clear the screen
pub fn clear(_args: &[&str]) {
    // ANSI escape sequence to clear screen and move cursor to top-left
    print!("\x1B[2J\x1B[H");
}

/// Reboot the system
pub fn reboot(_args: &[&str]) {
    println!("Rebooting system...");
    
    #[cfg(target_arch = "x86_64")]
    {
        // Triple fault to reboot (crude but works)
        unsafe {
            // Try keyboard controller reset first
            use x86_64::instructions::port::Port;
            let mut port = Port::<u8>::new(0x64);
            port.write(0xFE);
        }
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        // QEMU virt machine uses PSCI for power management
        // For now, just halt
        println!("(Reboot not implemented on AArch64 - please restart QEMU)");
    }
}
