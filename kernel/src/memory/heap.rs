//! Kernel Heap Allocator
//!
//! Provides dynamic memory allocation for kernel objects using a linked list allocator.
//! The heap is mapped to a fixed virtual address range.

use linked_list_allocator::LockedHeap;

/// Start of kernel heap virtual address
#[cfg(target_arch = "x86_64")]
pub const HEAP_START: usize = 0xFFFF_8880_0000_0000;

#[cfg(target_arch = "aarch64")]
pub const HEAP_START: usize = 0x4200_0000; // After kernel in QEMU virt RAM

/// Size of kernel heap (1MB initial)
pub const HEAP_SIZE: usize = 1024 * 1024;

/// Global allocator for the kernel
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Initialize the kernel heap
pub fn init() {
    // For now, we use a simple approach with the bootloader's frame allocator
    // The heap memory is already mapped by the bootloader in most cases
    
    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }
}

/// Get current heap usage statistics
pub fn stats() -> (usize, usize) {
    let allocator = ALLOCATOR.lock();
    let used = allocator.size() - allocator.free();
    let free = allocator.free();
    (used, free)
}
