//! Thread Scheduler
//!
//! Implements a preemptive priority-based round-robin scheduler with O(1) complexity.
//!
//! ## Priority Classes:
//! - Real-Time (0-31): Hard priority, runs until blocked or preempted by higher priority
//! - Normal (32-255): Dynamic priority with aging to prevent starvation

pub mod thread;
pub mod priority;

use spin::Mutex;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};

use priority::PriorityQueue;
use crate::arch::ArchContext;

// Import architecture-specific context switch functions
#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::context::{context_switch, context_switch_first};

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::context::{context_switch, context_switch_first};

// Re-export thread types for public API
pub use thread::{Thread, ThreadId, ThreadState};

/// Stack size for kernel threads (16KB)
const KERNEL_STACK_SIZE: usize = 16 * 1024;

/// Next available thread ID
static NEXT_TID: AtomicU64 = AtomicU64::new(1);

/// Whether the scheduler has started
static SCHEDULER_STARTED: AtomicBool = AtomicBool::new(false);

/// Timer ticks since boot
static TICKS: AtomicU64 = AtomicU64::new(0);

/// Global scheduler state
struct Scheduler {
    /// Currently running thread
    current: Option<Box<Thread>>,
    /// Priority queue for runnable threads
    ready_queue: PriorityQueue,
    /// Threads waiting for events
    blocked: Vec<Box<Thread>>,
}

impl Scheduler {
    const fn new() -> Self {
        Scheduler {
            current: None,
            ready_queue: PriorityQueue::new(),
            blocked: Vec::new(),
        }
    }
    
    /// Schedule the next thread to run
    fn schedule(&mut self) {
        // Get next thread from ready queue
        if let Some(mut next) = self.ready_queue.pop() {
            if let Some(mut current) = self.current.take() {
                // Save current thread and move to ready queue
                if current.state == ThreadState::Running {
                    current.state = ThreadState::Ready;
                    
                    // Context switch
                    let old_ctx = &mut current.context as *mut ArchContext;
                    let new_ctx = &next.context as *const ArchContext;
                    
                    self.ready_queue.push(current);
                    next.state = ThreadState::Running;
                    self.current = Some(next);
                    
                    unsafe {
                        context_switch(old_ctx, new_ctx);
                    }
                } else {
                    // Current thread is blocked/terminated, just switch
                    if current.state == ThreadState::Blocked {
                        self.blocked.push(current);
                    }
                    // Terminated threads are dropped
                    
                    next.state = ThreadState::Running;
                    let new_ctx = &next.context as *const ArchContext;
                    self.current = Some(next);
                    
                    unsafe {
                        context_switch_first(new_ctx);
                    }
                }
            } else {
                // No current thread, just start the next one
                next.state = ThreadState::Running;
                let new_ctx = &next.context as *const ArchContext;
                self.current = Some(next);
                
                unsafe {
                    context_switch_first(new_ctx);
                }
            }
        }
    }
    
    /// Yield the current thread
    fn yield_current(&mut self) {
        if self.current.is_some() && !self.ready_queue.is_empty() {
            self.schedule();
        }
    }
    
    /// Exit the current thread
    fn exit_current(&mut self, _exit_code: i32) {
        if let Some(mut current) = self.current.take() {
            current.state = ThreadState::Terminated;
            // Thread is dropped here
        }
        self.schedule();
    }
    
    /// Block the current thread
    fn block_current(&mut self) {
        if let Some(current) = self.current.as_mut() {
            current.state = ThreadState::Blocked;
        }
        self.schedule();
    }
    
    /// Unblock a thread by ID
    fn unblock(&mut self, tid: ThreadId) {
        if let Some(pos) = self.blocked.iter().position(|t| t.id == tid) {
            let mut thread = self.blocked.swap_remove(pos);
            thread.state = ThreadState::Ready;
            self.ready_queue.push(thread);
        }
    }
    
    /// Handle timer tick (preemption check)
    fn on_tick(&mut self) {
        if let Some(current) = self.current.as_mut() {
            current.time_slice = current.time_slice.saturating_sub(1);
            if current.time_slice == 0 {
                // Time slice expired, reset and reschedule
                current.time_slice = current.default_time_slice();
                self.schedule();
            }
        }
    }
}

/// Global scheduler instance
static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

/// Initialize the scheduler
pub fn init() {
    // Scheduler is ready, but not started until we have threads
}

/// Spawn a new kernel thread
pub fn spawn_thread(entry_point: usize, priority: u8) -> ThreadId {
    // Allocate stack for the new thread
    let stack = alloc::vec![0u8; KERNEL_STACK_SIZE].into_boxed_slice();
    let stack_top = stack.as_ptr() as usize + KERNEL_STACK_SIZE;
    
    // Create thread ID
    let tid = ThreadId(NEXT_TID.fetch_add(1, Ordering::Relaxed));
    
    // Create context
    let context = ArchContext::new_kernel(entry_point, stack_top);
    
    // Create thread
    let thread = Box::new(Thread::new(tid, priority, context, stack));
    
    // Add to ready queue
    {
        let mut scheduler = SCHEDULER.lock();
        scheduler.ready_queue.push(thread);
    }
    
    tid
}

/// Yield the current thread's time slice
pub fn yield_now() {
    if SCHEDULER_STARTED.load(Ordering::Relaxed) {
        SCHEDULER.lock().yield_current();
    }
}

/// Exit the current thread
pub fn exit_thread(exit_code: i32) -> ! {
    SCHEDULER.lock().exit_current(exit_code);
    
    // Should never reach here
    crate::idle_loop()
}

/// Block the current thread
pub fn block_current() {
    SCHEDULER.lock().block_current();
}

/// Unblock a thread
pub fn unblock(tid: ThreadId) {
    SCHEDULER.lock().unblock(tid);
}

/// Direct switch to a specific thread (L4-style IPC optimization)
/// 
/// This is used for fast IPC - when a sender has a message for a waiting
/// receiver, we switch directly to the receiver without going through
/// the scheduler. This reduces IPC latency significantly.
/// 
/// The current thread is blocked and the target thread runs immediately.
pub fn direct_switch_to(tid: ThreadId) {
    let mut scheduler = SCHEDULER.lock();
    
    // Find the target thread in blocked list
    if let Some(pos) = scheduler.blocked.iter().position(|t| t.id == tid) {
        let mut target = scheduler.blocked.swap_remove(pos);
        target.state = ThreadState::Running;
        
        if let Some(mut current) = scheduler.current.take() {
            // Block current thread
            current.state = ThreadState::Blocked;
            
            // Context switch directly to target
            let old_ctx = &mut current.context as *mut ArchContext;
            let new_ctx = &target.context as *const ArchContext;
            
            // Move current to blocked, target becomes current
            scheduler.blocked.push(current);
            scheduler.current = Some(target);
            
            // Drop the lock before context switch
            drop(scheduler);
            
            unsafe {
                context_switch(old_ctx, new_ctx);
            }
        } else {
            // No current thread (shouldn't happen in normal operation)
            let new_ctx = &target.context as *const ArchContext;
            scheduler.current = Some(target);
            
            drop(scheduler);
            
            unsafe {
                context_switch_first(new_ctx);
            }
        }
    } else {
        // Target not found in blocked list - fall back to regular blocking
        drop(scheduler);
        block_current();
    }
}

/// Get the current thread ID
pub fn current_tid() -> Option<ThreadId> {
    SCHEDULER.lock().current.as_ref().map(|t| t.id)
}

/// Timer interrupt handler - called from IDT/GIC
pub fn on_timer_tick() {
    let _ticks = TICKS.fetch_add(1, Ordering::Relaxed);
    
    // Start scheduler on first tick if we have threads
    if !SCHEDULER_STARTED.load(Ordering::Relaxed) {
        let scheduler = SCHEDULER.lock();
        if !scheduler.ready_queue.is_empty() {
            drop(scheduler);
            SCHEDULER_STARTED.store(true, Ordering::Relaxed);
            SCHEDULER.lock().schedule();
        }
    } else {
        // Regular tick - check for preemption
        SCHEDULER.lock().on_tick();
    }
}

/// Get uptime in ticks
pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Get the current thread's credentials
pub fn current_credentials() -> Option<crate::security::credentials::ProcessCredentials> {
    SCHEDULER.lock().current.as_ref().map(|t| t.credentials.clone())
}

/// Set the current thread's credentials
pub fn set_credentials(creds: crate::security::credentials::ProcessCredentials) -> Result<(), &'static str> {
    let mut scheduler = SCHEDULER.lock();
    if let Some(current) = scheduler.current.as_mut() {
        current.credentials = creds;
        Ok(())
    } else {
        Err("No current thread")
    }
}

/// Get the current thread's user ID
pub fn current_uid() -> Option<crate::security::identity::UserId> {
    SCHEDULER.lock().current.as_ref().map(|t| t.credentials.uid)
}

/// Get the current thread's effective user ID
pub fn current_euid() -> Option<crate::security::identity::UserId> {
    SCHEDULER.lock().current.as_ref().map(|t| t.credentials.euid)
}
