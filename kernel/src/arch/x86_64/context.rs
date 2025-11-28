//! CPU Context for x86_64
//!
//! Defines the architecture-specific context saved/restored during context switches.
//! This includes all general-purpose registers, instruction pointer, and flags.

use core::arch::naked_asm;

/// Architecture-specific CPU context
/// 
/// This structure stores all registers that must be preserved across
/// context switches. It's laid out to match the order of push/pop
/// operations in the context_switch assembly routine.
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct ArchContext {
    // Callee-saved registers (preserved across function calls)
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    
    // Instruction pointer (set by call instruction, restored by ret)
    pub rip: u64,
    
    // Stack pointer (saved separately)
    pub rsp: u64,
    
    // CPU flags
    pub rflags: u64,
    
    // Segment selectors (for user/kernel mode transitions)
    pub cs: u64,
    pub ss: u64,
    
    // Additional caller-saved registers (for full context saves)
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    
    // FPU/SSE state pointer (if needed)
    pub fxsave_area: u64,
}

impl ArchContext {
    /// Create a new context for a kernel thread
    pub fn new_kernel(entry_point: usize, stack_pointer: usize) -> Self {
        let mut ctx = Self::default();
        
        // Set up initial register state
        ctx.rip = entry_point as u64;
        ctx.rsp = stack_pointer as u64;
        ctx.rbp = stack_pointer as u64;
        
        // Enable interrupts in the new thread
        ctx.rflags = 0x200; // IF flag set
        
        // Kernel segment selectors (will be set properly after GDT init)
        ctx.cs = 0x08; // Kernel code segment
        ctx.ss = 0x10; // Kernel data segment
        
        ctx
    }
    
    /// Create a new context for a user thread
    pub fn new_user(entry_point: usize, stack_pointer: usize) -> Self {
        let mut ctx = Self::default();
        
        ctx.rip = entry_point as u64;
        ctx.rsp = stack_pointer as u64;
        ctx.rbp = stack_pointer as u64;
        ctx.rflags = 0x200; // IF flag set
        
        // User segment selectors with RPL=3
        ctx.cs = 0x23; // User code segment
        ctx.ss = 0x1B; // User data segment
        
        ctx
    }
}

/// Perform a context switch from the old context to the new context
/// 
/// # Safety
/// 
/// This function:
/// - Saves the current CPU state to `old`
/// - Loads the CPU state from `new`
/// - Transfers control to the new context
/// 
/// Both pointers must be valid and properly aligned.
#[unsafe(naked)]
pub unsafe extern "C" fn context_switch(old: *mut ArchContext, new: *const ArchContext) {
    // This is a naked function - we have full control over the stack
    naked_asm!(
        // Save callee-saved registers to old context
        "mov [rdi + 0*8], r15",
        "mov [rdi + 1*8], r14",
        "mov [rdi + 2*8], r13",
        "mov [rdi + 3*8], r12",
        "mov [rdi + 4*8], rbx",
        "mov [rdi + 5*8], rbp",
        
        // Save the return address (rip) - it's on the stack from the call
        "mov rax, [rsp]",
        "mov [rdi + 6*8], rax",
        
        // Save stack pointer
        "mov [rdi + 7*8], rsp",
        
        // Save flags
        "pushfq",
        "pop rax",
        "mov [rdi + 8*8], rax",
        
        // Load callee-saved registers from new context
        "mov r15, [rsi + 0*8]",
        "mov r14, [rsi + 1*8]",
        "mov r13, [rsi + 2*8]",
        "mov r12, [rsi + 3*8]",
        "mov rbx, [rsi + 4*8]",
        "mov rbp, [rsi + 5*8]",
        
        // Load stack pointer
        "mov rsp, [rsi + 7*8]",
        
        // Load flags
        "mov rax, [rsi + 8*8]",
        "push rax",
        "popfq",
        
        // Push return address and return (transfers control)
        "mov rax, [rsi + 6*8]",
        "push rax",
        "ret",
    );
}

/// Switch to a new context for the first time (no old context to save)
/// 
/// # Safety
/// 
/// The context pointer must be valid and the context must be properly initialized.
#[unsafe(naked)]
pub unsafe extern "C" fn context_switch_first(new: *const ArchContext) {
    naked_asm!(
        // Load callee-saved registers from new context
        "mov r15, [rdi + 0*8]",
        "mov r14, [rdi + 1*8]",
        "mov r13, [rdi + 2*8]",
        "mov r12, [rdi + 3*8]",
        "mov rbx, [rdi + 4*8]",
        "mov rbp, [rdi + 5*8]",
        
        // Load stack pointer
        "mov rsp, [rdi + 7*8]",
        
        // Load flags
        "mov rax, [rdi + 8*8]",
        "push rax",
        "popfq",
        
        // Jump to entry point
        "mov rax, [rdi + 6*8]",
        "push rax",
        "ret",
    );
}

