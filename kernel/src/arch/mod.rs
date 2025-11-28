//! Architecture-specific code
//!
//! This module contains all CPU architecture-specific implementations.
//! Currently supported: x86_64 and AArch64.

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

// Re-export architecture-specific items
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

/// Architecture-independent trait for context operations
pub trait Context: Default + Clone {
    /// Create a new kernel context
    fn new_kernel(entry_point: usize, stack_pointer: usize) -> Self;
    
    /// Create a new user context
    fn new_user(entry_point: usize, stack_pointer: usize) -> Self;
}
