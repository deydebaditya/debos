//! VirtQueue Implementation
//!
//! VirtQueues are the mechanism for bulk data transport in VirtIO devices.
//! Each queue consists of three parts:
//! - Descriptor Table: Describes buffers for data transfer
//! - Available Ring: Driver writes which descriptors are available to device
//! - Used Ring: Device writes which descriptors have been processed

use core::sync::atomic::{fence, Ordering};
use alloc::boxed::Box;
use alloc::vec::Vec;

/// Maximum number of descriptors in a queue
pub const MAX_QUEUE_SIZE: usize = 256;

/// Descriptor flags
pub mod desc_flags {
    pub const NEXT: u16 = 1;      // Buffer continues via next field
    pub const WRITE: u16 = 2;     // Buffer is write-only (for device)
    pub const INDIRECT: u16 = 4;  // Buffer contains list of descriptors
}

/// A VirtQueue descriptor
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Descriptor {
    /// Physical address of the buffer
    pub addr: u64,
    /// Length of the buffer
    pub len: u32,
    /// Descriptor flags
    pub flags: u16,
    /// Next descriptor if NEXT flag is set
    pub next: u16,
}

/// Available ring structure
#[repr(C)]
pub struct AvailableRing {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; MAX_QUEUE_SIZE],
    pub used_event: u16,
}

/// Used ring element
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct UsedElement {
    pub id: u32,
    pub len: u32,
}

/// Used ring structure
#[repr(C)]
pub struct UsedRing {
    pub flags: u16,
    pub idx: u16,
    pub ring: [UsedElement; MAX_QUEUE_SIZE],
    pub avail_event: u16,
}

/// A VirtQueue for VirtIO communication
pub struct VirtQueue {
    /// Queue index
    pub index: u16,
    /// Queue size (number of descriptors)
    pub size: u16,
    /// Descriptor table
    descriptors: Box<[Descriptor; MAX_QUEUE_SIZE]>,
    /// Available ring
    available: Box<AvailableRing>,
    /// Used ring  
    used: Box<UsedRing>,
    /// Free descriptor list head
    free_head: u16,
    /// Number of free descriptors
    num_free: u16,
    /// Last seen used index
    last_used_idx: u16,
    /// Pending buffers (descriptor chain heads waiting for completion)
    pending: Vec<u16>,
}

impl VirtQueue {
    /// Create a new VirtQueue
    pub fn new(index: u16, size: u16) -> Self {
        let size = size.min(MAX_QUEUE_SIZE as u16);
        
        // Allocate descriptor table
        let mut descriptors = Box::new([Descriptor::default(); MAX_QUEUE_SIZE]);
        
        // Initialize free list (chain all descriptors)
        for i in 0..(size - 1) {
            descriptors[i as usize].next = i + 1;
        }
        
        // Allocate available ring
        let available = Box::new(AvailableRing {
            flags: 0,
            idx: 0,
            ring: [0; MAX_QUEUE_SIZE],
            used_event: 0,
        });
        
        // Allocate used ring
        let used = Box::new(UsedRing {
            flags: 0,
            idx: 0,
            ring: [UsedElement::default(); MAX_QUEUE_SIZE],
            avail_event: 0,
        });
        
        VirtQueue {
            index,
            size,
            descriptors,
            available,
            used,
            free_head: 0,
            num_free: size,
            last_used_idx: 0,
            pending: Vec::new(),
        }
    }
    
    /// Get physical addresses for device configuration
    pub fn descriptor_area(&self) -> u64 {
        self.descriptors.as_ptr() as u64
    }
    
    pub fn driver_area(&self) -> u64 {
        self.available.as_ref() as *const AvailableRing as u64
    }
    
    pub fn device_area(&self) -> u64 {
        self.used.as_ref() as *const UsedRing as u64
    }
    
    /// Check if queue is full
    pub fn is_full(&self) -> bool {
        self.num_free == 0
    }
    
    /// Check if there are pending requests
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
    
    /// Allocate a descriptor from the free list
    fn alloc_descriptor(&mut self) -> Option<u16> {
        if self.num_free == 0 {
            return None;
        }
        
        let desc_idx = self.free_head;
        self.free_head = self.descriptors[desc_idx as usize].next;
        self.num_free -= 1;
        
        Some(desc_idx)
    }
    
    /// Free a descriptor chain back to the free list
    fn free_chain(&mut self, mut head: u16) {
        loop {
            let desc = &mut self.descriptors[head as usize];
            let next = desc.next;
            let has_next = desc.flags & desc_flags::NEXT != 0;
            
            // Add to free list
            desc.next = self.free_head;
            desc.flags = 0;
            self.free_head = head;
            self.num_free += 1;
            
            if !has_next {
                break;
            }
            head = next;
        }
    }
    
    /// Add a buffer chain to the queue
    /// 
    /// Returns the descriptor chain head index
    pub fn add_buffer(&mut self, bufs: &[(u64, u32, bool)]) -> Result<u16, &'static str> {
        if bufs.is_empty() {
            return Err("Empty buffer list");
        }
        
        if bufs.len() > self.num_free as usize {
            return Err("Not enough descriptors");
        }
        
        let head = self.alloc_descriptor().ok_or("No free descriptors")?;
        let mut prev = head;
        
        for (i, &(addr, len, device_write)) in bufs.iter().enumerate() {
            let desc_idx = if i == 0 { head } else {
                let idx = self.alloc_descriptor().ok_or("No free descriptors")?;
                self.descriptors[prev as usize].next = idx;
                self.descriptors[prev as usize].flags |= desc_flags::NEXT;
                idx
            };
            
            let desc = &mut self.descriptors[desc_idx as usize];
            desc.addr = addr;
            desc.len = len;
            desc.flags = if device_write { desc_flags::WRITE } else { 0 };
            
            prev = desc_idx;
        }
        
        // Add to available ring
        let avail_idx = self.available.idx as usize % self.size as usize;
        self.available.ring[avail_idx] = head;
        
        // Memory barrier before updating idx
        fence(Ordering::SeqCst);
        
        self.available.idx = self.available.idx.wrapping_add(1);
        
        // Track pending request
        self.pending.push(head);
        
        Ok(head)
    }
    
    /// Check for completed requests and return their descriptor heads
    pub fn poll_used(&mut self) -> Option<(u16, u32)> {
        // Memory barrier
        fence(Ordering::SeqCst);
        
        if self.last_used_idx == self.used.idx {
            return None;
        }
        
        let used_idx = self.last_used_idx as usize % self.size as usize;
        let used_elem = self.used.ring[used_idx];
        
        self.last_used_idx = self.last_used_idx.wrapping_add(1);
        
        // Remove from pending and free the chain
        if let Some(pos) = self.pending.iter().position(|&x| x == used_elem.id as u16) {
            self.pending.remove(pos);
        }
        
        let head = used_elem.id as u16;
        self.free_chain(head);
        
        Some((head, used_elem.len))
    }
    
    /// Notify the device that new buffers are available
    pub fn notify_available(&self) -> bool {
        // Check if notification is needed
        // For now, always notify (no event suppression)
        true
    }
}

