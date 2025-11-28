//! libdebos - DebOS Standard Library
//!
//! Provides user-space abstractions for system calls and IPC.

#![no_std]

pub mod syscall;
pub mod thread;
pub mod ipc;
pub mod fs;
pub mod net;

// Re-export common types
pub use thread::Thread;
