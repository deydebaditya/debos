//! Memory Management
//!
//! This module handles all memory-related functionality:
//! - Physical memory allocation (Buddy allocator)
//! - Kernel heap
//! - Virtual memory management

pub mod buddy;
pub mod heap;

use crate::println;

// ============================================================================
// x86_64-specific memory initialization
// ============================================================================

#[cfg(target_arch = "x86_64")]
use bootloader_api::BootInfo;

#[cfg(target_arch = "x86_64")]
use x86_64::VirtAddr;

/// Initialize memory management for x86_64
#[cfg(target_arch = "x86_64")]
pub fn init_x86(boot_info: &'static mut BootInfo) {
    // Get the physical memory offset from bootloader
    let physical_memory_offset = boot_info
        .physical_memory_offset
        .into_option()
        .expect("Physical memory offset not provided by bootloader");
    
    let phys_mem_offset = VirtAddr::new(physical_memory_offset);
    
    // Get memory regions
    let memory_regions = &boot_info.memory_regions;
    
    // Calculate total usable memory
    let total_memory: u64 = memory_regions
        .iter()
        .filter(|r| r.kind == bootloader_api::info::MemoryRegionKind::Usable)
        .map(|r| r.end - r.start)
        .sum();
    
    println!("  Total usable memory: {} MB", total_memory / 1024 / 1024);
    
    // Initialize paging
    unsafe {
        crate::arch::x86_64::paging::init(phys_mem_offset, memory_regions);
    }
    println!("  Paging initialized");
    
    // Initialize heap
    heap::init();
    println!("  Heap initialized ({}KB)", heap::HEAP_SIZE / 1024);
    
    // Initialize buddy allocator for frame allocation
    buddy::init_x86(memory_regions);
    println!("  Buddy allocator initialized");
}

// ============================================================================
// AArch64-specific memory initialization
// ============================================================================

/// Initialize memory management for AArch64
#[cfg(target_arch = "aarch64")]
pub fn init_aarch64() {
    // For QEMU virt machine, we have a known memory layout
    // RAM starts at 0x4000_0000 and is typically 512MB
    const RAM_START: usize = 0x4000_0000;
    const RAM_SIZE: usize = 512 * 1024 * 1024; // 512MB
    
    println!("  Total usable memory: {} MB", RAM_SIZE / 1024 / 1024);
    
    // Initialize MMU
    crate::arch::aarch64::mmu::init();
    println!("  MMU initialized");
    
    // Initialize heap
    heap::init();
    println!("  Heap initialized ({}KB)", heap::HEAP_SIZE / 1024);
    
    // Initialize buddy allocator
    buddy::init_aarch64(RAM_START, RAM_SIZE);
    println!("  Buddy allocator initialized");
}

// ============================================================================
// Common memory functions
// ============================================================================

/// Allocate physical memory frames
pub fn allocate_frames(count: usize) -> Option<usize> {
    buddy::allocate(count)
}

/// Free physical memory frames
pub fn free_frames(addr: usize, count: usize) {
    buddy::free(addr, count);
}
