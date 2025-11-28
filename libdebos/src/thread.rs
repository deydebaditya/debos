//! Thread API

use crate::syscall::{self, *};

/// Thread handle
pub struct Thread {
    id: u64,
}

impl Thread {
    /// Spawn a new thread
    pub fn spawn(entry: fn()) -> Result<Thread, i64> {
        let result = unsafe {
            syscall::syscall(
                SYS_THREAD_SPAWN,
                entry as u64,
                0, // stack (kernel will allocate)
                128, // priority
                0, // capability
                0,
                0,
            )
        };
        
        if result < 0 {
            Err(result)
        } else {
            Ok(Thread { id: result as u64 })
        }
    }
    
    /// Yield the current thread's time slice
    pub fn yield_now() {
        unsafe {
            syscall::syscall(SYS_THREAD_YIELD, 0, 0, 0, 0, 0, 0);
        }
    }
    
    /// Exit the current thread
    pub fn exit(code: i32) -> ! {
        unsafe {
            syscall::syscall(SYS_THREAD_EXIT, code as u64, 0, 0, 0, 0, 0);
        }
        loop {}
    }
    
    /// Get the current thread ID
    pub fn current_id() -> u64 {
        unsafe {
            syscall::syscall(SYS_THREAD_GET_ID, 0, 0, 0, 0, 0, 0) as u64
        }
    }
}

