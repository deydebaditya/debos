//! AArch64 CPU Context
//!
//! Defines the architecture-specific context saved/restored during context switches.

use core::arch::{asm, global_asm};

/// Architecture-specific CPU context for AArch64
///
/// Contains all registers that must be preserved across context switches.
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct ArchContext {
    // Callee-saved registers (X19-X30)
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64,  // Frame pointer
    pub x30: u64,  // Link register (return address)
    
    // Stack pointer
    pub sp: u64,
    
    // Program counter (entry point for new threads)
    pub pc: u64,
    
    // Saved Program Status Register
    pub spsr: u64,
    
    // Thread-local storage pointer
    pub tpidr: u64,
}

impl ArchContext {
    /// Create a new context for a kernel thread
    pub fn new_kernel(entry_point: usize, stack_pointer: usize) -> Self {
        let mut ctx = Self::default();
        
        ctx.pc = entry_point as u64;
        ctx.sp = stack_pointer as u64;
        ctx.x29 = stack_pointer as u64;  // Frame pointer
        
        // SPSR: EL1h (handler mode with SP_EL1), interrupts enabled
        // DAIF = 0 (no exceptions masked)
        ctx.spsr = 0b0000_0000_0000_0101; // EL1h, DAIF clear
        
        ctx
    }
    
    /// Create a new context for a user thread
    pub fn new_user(entry_point: usize, stack_pointer: usize) -> Self {
        let mut ctx = Self::default();
        
        ctx.pc = entry_point as u64;
        ctx.sp = stack_pointer as u64;
        ctx.x29 = stack_pointer as u64;
        
        // SPSR: EL0t (user mode with SP_EL0), interrupts enabled
        ctx.spsr = 0b0000_0000_0000_0000; // EL0t
        
        ctx
    }
}

impl crate::arch::Context for ArchContext {
    fn new_kernel(entry_point: usize, stack_pointer: usize) -> Self {
        Self::new_kernel(entry_point, stack_pointer)
    }
    
    fn new_user(entry_point: usize, stack_pointer: usize) -> Self {
        Self::new_user(entry_point, stack_pointer)
    }
}

// Context switch implementation in assembly
global_asm!(
    r#"
.global context_switch
.global context_switch_first

// context_switch(old: *mut ArchContext, new: *const ArchContext)
// x0 = old context pointer
// x1 = new context pointer
context_switch:
    // Save callee-saved registers to old context
    stp     x19, x20, [x0, #(0*8)]
    stp     x21, x22, [x0, #(2*8)]
    stp     x23, x24, [x0, #(4*8)]
    stp     x25, x26, [x0, #(6*8)]
    stp     x27, x28, [x0, #(8*8)]
    stp     x29, x30, [x0, #(10*8)]
    
    // Save stack pointer
    mov     x2, sp
    str     x2, [x0, #(12*8)]
    
    // Save return address as PC
    str     x30, [x0, #(13*8)]
    
    // Fall through to load new context
    
context_switch_first:
    // x1 = new context pointer (or x0 for context_switch_first)
    cbz     x0, 1f
    mov     x1, x0
1:
    
    // Load callee-saved registers from new context
    ldp     x19, x20, [x1, #(0*8)]
    ldp     x21, x22, [x1, #(2*8)]
    ldp     x23, x24, [x1, #(4*8)]
    ldp     x25, x26, [x1, #(6*8)]
    ldp     x27, x28, [x1, #(8*8)]
    ldp     x29, x30, [x1, #(10*8)]
    
    // Load stack pointer
    ldr     x2, [x1, #(12*8)]
    mov     sp, x2
    
    // Load PC and jump
    ldr     x2, [x1, #(13*8)]
    br      x2
"#
);

extern "C" {
    /// Perform a context switch from old to new context
    pub fn context_switch(old: *mut ArchContext, new: *const ArchContext);
    
    /// Switch to a new context for the first time
    pub fn context_switch_first(new: *const ArchContext) -> !;
}

