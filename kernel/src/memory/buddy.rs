//! Buddy Allocator for Physical Memory
//!
//! The buddy allocator manages physical page frames using a binary tree structure.
//! It supports allocation of contiguous power-of-2 sized blocks from 4KB to 4MB.
//!
//! ## How it works:
//! - Memory is divided into blocks of size 2^n pages
//! - Each order has a free list of available blocks
//! - When allocating, we find the smallest order that fits
//! - When freeing, we try to merge (coalesce) with the "buddy" block

#[cfg(target_arch = "x86_64")]
use bootloader_api::info::{MemoryRegionKind, MemoryRegions};

use spin::Mutex;
use alloc::vec::Vec;
use core::cmp::min;

/// Page size (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Maximum order (2^10 pages = 4MB blocks)
pub const MAX_ORDER: usize = 10;

/// Buddy allocator state
pub struct BuddyAllocator {
    /// Free lists for each order (order 0 = 4KB, order 10 = 4MB)
    free_lists: [Vec<usize>; MAX_ORDER + 1],
    /// Total pages managed
    total_pages: usize,
    /// Free pages available
    free_pages: usize,
}

impl BuddyAllocator {
    /// Create a new empty buddy allocator
    const fn new() -> Self {
        BuddyAllocator {
            free_lists: [
                Vec::new(), Vec::new(), Vec::new(), Vec::new(),
                Vec::new(), Vec::new(), Vec::new(), Vec::new(),
                Vec::new(), Vec::new(), Vec::new(),
            ],
            total_pages: 0,
            free_pages: 0,
        }
    }
    
    /// Initialize with memory regions from bootloader (x86_64)
    #[cfg(target_arch = "x86_64")]
    fn init_from_regions(&mut self, memory_regions: &MemoryRegions) {
        for region in memory_regions.iter() {
            if region.kind == MemoryRegionKind::Usable {
                let start = region.start as usize;
                let end = region.end as usize;
                
                // Align start up to page boundary
                let aligned_start = (start + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
                // Align end down to page boundary
                let aligned_end = end & !(PAGE_SIZE - 1);
                
                if aligned_end > aligned_start {
                    self.add_region(aligned_start, aligned_end);
                }
            }
        }
    }
    
    /// Initialize with a memory range (AArch64)
    fn init_from_range(&mut self, start: usize, size: usize) {
        // Reserve first 16MB for kernel and early allocations
        let kernel_reserved = 16 * 1024 * 1024;
        let usable_start = start + kernel_reserved;
        let usable_end = start + size;
        
        if usable_end > usable_start {
            self.add_region(usable_start, usable_end);
        }
    }
    
    /// Add a memory region to the allocator
    fn add_region(&mut self, start: usize, end: usize) {
        let mut current = start;
        
        while current < end {
            // Find the largest block we can add at this address
            let remaining = end - current;
            let max_pages = remaining / PAGE_SIZE;
            
            // Find highest order that fits
            let mut order = 0;
            while order < MAX_ORDER && (1 << (order + 1)) <= max_pages {
                // Check alignment: block must be aligned to its size
                let block_size = (1 << (order + 1)) * PAGE_SIZE;
                if current % block_size == 0 {
                    order += 1;
                } else {
                    break;
                }
            }
            
            let pages = 1 << order;
            self.free_lists[order].push(current);
            self.total_pages += pages;
            self.free_pages += pages;
            
            current += pages * PAGE_SIZE;
        }
    }
    
    /// Allocate contiguous pages
    /// 
    /// Returns the physical address of the first page, or None if allocation fails.
    pub fn allocate(&mut self, pages: usize) -> Option<usize> {
        if pages == 0 {
            return None;
        }
        
        // Find the order needed (smallest power of 2 >= pages)
        let order = pages.next_power_of_two().trailing_zeros() as usize;
        let order = min(order, MAX_ORDER);
        
        // Find a block of sufficient size
        self.allocate_order(order)
    }
    
    /// Allocate a block of the given order
    fn allocate_order(&mut self, order: usize) -> Option<usize> {
        // Try to find a block at this order
        if let Some(addr) = self.free_lists[order].pop() {
            self.free_pages -= 1 << order;
            return Some(addr);
        }
        
        // No block at this order, try to split a larger block
        if order < MAX_ORDER {
            if let Some(larger_addr) = self.allocate_order(order + 1) {
                // Split the larger block
                let block_size = (1 << order) * PAGE_SIZE;
                let buddy_addr = larger_addr + block_size;
                
                // Add buddy to free list
                self.free_lists[order].push(buddy_addr);
                self.free_pages += 1 << order;
                
                return Some(larger_addr);
            }
        }
        
        None
    }
    
    /// Free previously allocated pages
    pub fn free(&mut self, addr: usize, pages: usize) {
        if pages == 0 {
            return;
        }
        
        let order = pages.next_power_of_two().trailing_zeros() as usize;
        let order = min(order, MAX_ORDER);
        
        self.free_order(addr, order);
    }
    
    /// Free a block of the given order, attempting to coalesce with buddy
    fn free_order(&mut self, addr: usize, order: usize) {
        if order >= MAX_ORDER {
            // Can't coalesce at max order
            self.free_lists[order].push(addr);
            self.free_pages += 1 << order;
            return;
        }
        
        // Calculate buddy address
        let block_size = (1 << order) * PAGE_SIZE;
        let buddy_addr = addr ^ block_size;
        
        // Try to find buddy in free list
        if let Some(pos) = self.free_lists[order].iter().position(|&a| a == buddy_addr) {
            // Found buddy, remove it and coalesce
            self.free_lists[order].swap_remove(pos);
            self.free_pages -= 1 << order;
            
            // Recursively free the merged block
            let merged_addr = min(addr, buddy_addr);
            self.free_order(merged_addr, order + 1);
        } else {
            // No buddy, just add to free list
            self.free_lists[order].push(addr);
            self.free_pages += 1 << order;
        }
    }
    
    /// Get statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.total_pages * PAGE_SIZE, self.free_pages * PAGE_SIZE)
    }
}

/// Global buddy allocator instance
static ALLOCATOR: Mutex<BuddyAllocator> = Mutex::new(BuddyAllocator::new());

/// Initialize the buddy allocator with memory regions (x86_64)
#[cfg(target_arch = "x86_64")]
pub fn init_x86(memory_regions: &MemoryRegions) {
    ALLOCATOR.lock().init_from_regions(memory_regions);
}

/// Initialize the buddy allocator with a memory range (AArch64)
#[cfg(target_arch = "aarch64")]
pub fn init_aarch64(start: usize, size: usize) {
    ALLOCATOR.lock().init_from_range(start, size);
}

/// Allocate contiguous physical pages
pub fn allocate(pages: usize) -> Option<usize> {
    ALLOCATOR.lock().allocate(pages)
}

/// Free physical pages
pub fn free(addr: usize, pages: usize) {
    ALLOCATOR.lock().free(addr, pages);
}

/// Get allocator statistics (total bytes, free bytes)
pub fn stats() -> (usize, usize) {
    ALLOCATOR.lock().stats()
}
