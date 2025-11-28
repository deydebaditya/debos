//! x86_64 Architecture Support
//!
//! This module provides all x86_64-specific functionality:
//! - GDT (Global Descriptor Table)
//! - IDT (Interrupt Descriptor Table)
//! - Paging (4-level page tables)
//! - Context switching
//! - Serial output

pub mod gdt;
pub mod idt;
pub mod paging;
pub mod context;
pub mod serial;

pub use context::ArchContext;

