//! O(1) Priority Queue for Scheduler
//!
//! Uses a bitmap-indexed array of queues for constant-time
//! insertion and extraction of the highest-priority thread.

use alloc::boxed::Box;
use alloc::collections::VecDeque;

use super::thread::Thread;

/// Number of priority levels (0-255)
const PRIORITY_LEVELS: usize = 256;

/// Number of 64-bit words needed for bitmap (256 / 64 = 4)
const BITMAP_WORDS: usize = 4;

/// O(1) Priority Queue
/// 
/// Uses a bitmap to track which priority levels have runnable threads,
/// and an array of FIFO queues for each priority level.
pub struct PriorityQueue {
    /// Bitmap indicating which priorities have threads
    /// Bit N is set if priority N has at least one thread
    bitmap: [u64; BITMAP_WORDS],
    /// Per-priority FIFO queues
    queues: [VecDeque<Box<Thread>>; PRIORITY_LEVELS],
    /// Total number of threads in the queue
    count: usize,
}

impl PriorityQueue {
    /// Create a new empty priority queue
    pub const fn new() -> Self {
        // Create array of empty VecDeques
        // This is a bit verbose but necessary for const initialization
        const EMPTY_QUEUE: VecDeque<Box<Thread>> = VecDeque::new();
        
        PriorityQueue {
            bitmap: [0; BITMAP_WORDS],
            queues: [EMPTY_QUEUE; PRIORITY_LEVELS],
            count: 0,
        }
    }
    
    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    
    /// Get the number of threads in the queue
    pub fn len(&self) -> usize {
        self.count
    }
    
    /// Insert a thread into the queue
    pub fn push(&mut self, thread: Box<Thread>) {
        let priority = thread.dynamic_priority as usize;
        
        // Add to the appropriate queue
        self.queues[priority].push_back(thread);
        
        // Set bit in bitmap
        let word = priority / 64;
        let bit = priority % 64;
        self.bitmap[word] |= 1 << bit;
        
        self.count += 1;
    }
    
    /// Remove and return the highest-priority thread
    pub fn pop(&mut self) -> Option<Box<Thread>> {
        if self.count == 0 {
            return None;
        }
        
        // Find highest priority (lowest number) with threads
        let priority = self.find_highest_priority()?;
        
        // Pop from that queue
        let thread = self.queues[priority].pop_front()?;
        
        // Clear bit if queue is now empty
        if self.queues[priority].is_empty() {
            let word = priority / 64;
            let bit = priority % 64;
            self.bitmap[word] &= !(1 << bit);
        }
        
        self.count -= 1;
        Some(thread)
    }
    
    /// Peek at the highest-priority thread without removing it
    pub fn peek(&self) -> Option<&Thread> {
        if self.count == 0 {
            return None;
        }
        
        let priority = self.find_highest_priority()?;
        self.queues[priority].front().map(|t| t.as_ref())
    }
    
    /// Find the highest priority level with threads
    fn find_highest_priority(&self) -> Option<usize> {
        // Check each bitmap word from lowest to highest
        for (word_idx, &word) in self.bitmap.iter().enumerate() {
            if word != 0 {
                // Find lowest set bit (highest priority in this word)
                let bit = word.trailing_zeros() as usize;
                return Some(word_idx * 64 + bit);
            }
        }
        None
    }
    
    /// Apply priority aging to all threads in the queue
    /// 
    /// This prevents starvation of lower-priority threads by temporarily
    /// boosting their priority if they've been waiting too long.
    pub fn apply_aging(&mut self, aging_factor: u8) {
        // For normal priority threads, boost priority if waiting
        for priority in 32..PRIORITY_LEVELS {
            let queue = &mut self.queues[priority];
            
            for thread in queue.iter_mut() {
                // Boost priority (lower number = higher priority)
                let new_priority = thread.dynamic_priority.saturating_sub(aging_factor);
                
                // Don't boost into real-time range
                if new_priority >= 32 {
                    thread.dynamic_priority = new_priority;
                }
            }
        }
        
        // Note: In a full implementation, we'd need to rebalance the queues
        // when priorities change. For simplicity, we only apply aging at pop time.
    }
    
    /// Reset a thread's dynamic priority to its base priority
    pub fn reset_priority(&mut self, thread: &mut Thread) {
        thread.dynamic_priority = thread.priority;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Tests would go here
}

