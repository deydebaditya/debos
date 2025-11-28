//! AArch64 Exception Handling
//!
//! AArch64 has 4 Exception Levels (EL0-EL3) and different exception types:
//! - Synchronous: Instruction aborts, data aborts, SVC calls
//! - IRQ: Normal interrupts
//! - FIQ: Fast interrupts  
//! - SError: System errors

use core::arch::global_asm;

/// Exception context saved on the stack
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ExceptionContext {
    /// General purpose registers X0-X30
    pub gpr: [u64; 31],
    /// Exception Link Register (return address)
    pub elr: u64,
    /// Saved Program Status Register
    pub spsr: u64,
    /// Exception Syndrome Register
    pub esr: u64,
    /// Fault Address Register
    pub far: u64,
}

/// Exception vector table
/// Each entry is 128 bytes (32 instructions) apart
global_asm!(
    r#"
.section .text
.balign 0x800
.global exception_vector_table
exception_vector_table:

// Current EL with SP_EL0
.balign 0x80
    b       sync_current_el_sp0
.balign 0x80
    b       irq_current_el_sp0
.balign 0x80
    b       fiq_current_el_sp0
.balign 0x80
    b       serror_current_el_sp0

// Current EL with SP_ELx
.balign 0x80
    b       sync_current_el_spx
.balign 0x80
    b       irq_current_el_spx
.balign 0x80
    b       fiq_current_el_spx
.balign 0x80
    b       serror_current_el_spx

// Lower EL, AArch64
.balign 0x80
    b       sync_lower_el_aarch64
.balign 0x80
    b       irq_lower_el_aarch64
.balign 0x80
    b       fiq_lower_el_aarch64
.balign 0x80
    b       serror_lower_el_aarch64

// Lower EL, AArch32
.balign 0x80
    b       sync_lower_el_aarch32
.balign 0x80
    b       irq_lower_el_aarch32
.balign 0x80
    b       fiq_lower_el_aarch32
.balign 0x80
    b       serror_lower_el_aarch32

// Exception handlers
.macro SAVE_CONTEXT
    sub     sp, sp, #(8 * 35)
    stp     x0, x1, [sp, #(8 * 0)]
    stp     x2, x3, [sp, #(8 * 2)]
    stp     x4, x5, [sp, #(8 * 4)]
    stp     x6, x7, [sp, #(8 * 6)]
    stp     x8, x9, [sp, #(8 * 8)]
    stp     x10, x11, [sp, #(8 * 10)]
    stp     x12, x13, [sp, #(8 * 12)]
    stp     x14, x15, [sp, #(8 * 14)]
    stp     x16, x17, [sp, #(8 * 16)]
    stp     x18, x19, [sp, #(8 * 18)]
    stp     x20, x21, [sp, #(8 * 20)]
    stp     x22, x23, [sp, #(8 * 22)]
    stp     x24, x25, [sp, #(8 * 24)]
    stp     x26, x27, [sp, #(8 * 26)]
    stp     x28, x29, [sp, #(8 * 28)]
    str     x30, [sp, #(8 * 30)]
    
    mrs     x0, ELR_EL1
    mrs     x1, SPSR_EL1
    mrs     x2, ESR_EL1
    mrs     x3, FAR_EL1
    stp     x0, x1, [sp, #(8 * 31)]
    stp     x2, x3, [sp, #(8 * 33)]
.endm

.macro RESTORE_CONTEXT
    ldp     x0, x1, [sp, #(8 * 31)]
    msr     ELR_EL1, x0
    msr     SPSR_EL1, x1
    
    ldp     x0, x1, [sp, #(8 * 0)]
    ldp     x2, x3, [sp, #(8 * 2)]
    ldp     x4, x5, [sp, #(8 * 4)]
    ldp     x6, x7, [sp, #(8 * 6)]
    ldp     x8, x9, [sp, #(8 * 8)]
    ldp     x10, x11, [sp, #(8 * 10)]
    ldp     x12, x13, [sp, #(8 * 12)]
    ldp     x14, x15, [sp, #(8 * 14)]
    ldp     x16, x17, [sp, #(8 * 16)]
    ldp     x18, x19, [sp, #(8 * 18)]
    ldp     x20, x21, [sp, #(8 * 20)]
    ldp     x22, x23, [sp, #(8 * 22)]
    ldp     x24, x25, [sp, #(8 * 24)]
    ldp     x26, x27, [sp, #(8 * 26)]
    ldp     x28, x29, [sp, #(8 * 28)]
    ldr     x30, [sp, #(8 * 30)]
    add     sp, sp, #(8 * 35)
.endm

sync_current_el_sp0:
sync_current_el_spx:
sync_lower_el_aarch64:
sync_lower_el_aarch32:
    SAVE_CONTEXT
    mov     x0, sp
    bl      handle_sync_exception
    RESTORE_CONTEXT
    eret

irq_current_el_sp0:
irq_current_el_spx:
irq_lower_el_aarch64:
irq_lower_el_aarch32:
    SAVE_CONTEXT
    mov     x0, sp
    bl      handle_irq
    RESTORE_CONTEXT
    eret

fiq_current_el_sp0:
fiq_current_el_spx:
fiq_lower_el_aarch64:
fiq_lower_el_aarch32:
    SAVE_CONTEXT
    mov     x0, sp
    bl      handle_fiq
    RESTORE_CONTEXT
    eret

serror_current_el_sp0:
serror_current_el_spx:
serror_lower_el_aarch64:
serror_lower_el_aarch32:
    SAVE_CONTEXT
    mov     x0, sp
    bl      handle_serror
    RESTORE_CONTEXT
    eret
"#
);

extern "C" {
    static exception_vector_table: u64;
}

/// Initialize exception handling
pub fn init() {
    unsafe {
        // Set the Vector Base Address Register
        let vbar = &exception_vector_table as *const _ as u64;
        core::arch::asm!("msr VBAR_EL1, {}", in(reg) vbar);
    }
    
    crate::println!("[OK] Exception vectors installed");
}

/// Handle synchronous exceptions
#[no_mangle]
extern "C" fn handle_sync_exception(ctx: &ExceptionContext) {
    let esr = ctx.esr;
    let ec = (esr >> 26) & 0x3F;  // Exception Class
    let iss = esr & 0x1FFFFFF;     // Instruction Specific Syndrome
    
    match ec {
        0b010101 => {
            // SVC instruction from AArch64 (system call)
            handle_svc(ctx, iss as u16);
        }
        0b100000 | 0b100001 => {
            // Instruction Abort
            crate::println!("EXCEPTION: Instruction Abort at 0x{:016x}", ctx.elr);
            crate::println!("FAR: 0x{:016x}, ESR: 0x{:08x}", ctx.far, esr);
            panic!("Instruction Abort");
        }
        0b100100 | 0b100101 => {
            // Data Abort
            crate::println!("EXCEPTION: Data Abort at 0x{:016x}", ctx.elr);
            crate::println!("FAR: 0x{:016x}, ESR: 0x{:08x}", ctx.far, esr);
            panic!("Data Abort");
        }
        _ => {
            crate::println!("EXCEPTION: Unknown synchronous exception");
            crate::println!("EC: 0x{:02x}, ISS: 0x{:06x}", ec, iss);
            crate::println!("ELR: 0x{:016x}", ctx.elr);
            panic!("Unknown exception");
        }
    }
}

/// Handle SVC (system call)
fn handle_svc(_ctx: &ExceptionContext, _svc_num: u16) {
    // TODO: Implement system call handling
    crate::println!("[SVC] System call");
}

/// Handle IRQ
#[no_mangle]
extern "C" fn handle_irq(_ctx: &ExceptionContext) {
    // Read interrupt ID from GIC
    super::gic::handle_interrupt();
}

/// Handle FIQ
#[no_mangle]
extern "C" fn handle_fiq(_ctx: &ExceptionContext) {
    crate::println!("EXCEPTION: FIQ");
}

/// Handle SError
#[no_mangle]
extern "C" fn handle_serror(ctx: &ExceptionContext) {
    crate::println!("EXCEPTION: SError at 0x{:016x}", ctx.elr);
    panic!("System Error");
}

