//! Global Descriptor Table (GDT)
//!
//! The GDT defines memory segments and privilege levels for x86_64.
//! In 64-bit long mode, segmentation is mostly disabled, but we still need:
//! - Code segments for kernel (Ring 0) and user (Ring 3)
//! - Data segments for kernel and user
//! - Task State Segment (TSS) for stack switching on interrupts

use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

/// Size of the interrupt stack (16KB)
pub const INTERRUPT_STACK_SIZE: usize = 4096 * 4;

/// Index for the double fault stack in the IST
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    /// Task State Segment - provides interrupt stacks
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        
        // Set up the Interrupt Stack Table (IST)
        // IST[0] is used for double faults to ensure we have a valid stack
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            static mut STACK: [u8; INTERRUPT_STACK_SIZE] = [0; INTERRUPT_STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(&raw const STACK as *const u8);
            let stack_end = stack_start + INTERRUPT_STACK_SIZE as u64;
            stack_end
        };
        
        // Privilege stack table - stack used when switching from Ring 3 to Ring 0
        tss.privilege_stack_table[0] = {
            static mut STACK: [u8; INTERRUPT_STACK_SIZE] = [0; INTERRUPT_STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(&raw const STACK as *const u8);
            let stack_end = stack_start + INTERRUPT_STACK_SIZE as u64;
            stack_end
        };
        
        tss
    };
    
    /// Global Descriptor Table
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        
        // Kernel segments (Ring 0)
        let kernel_code_selector = gdt.append(Descriptor::kernel_code_segment());
        let kernel_data_selector = gdt.append(Descriptor::kernel_data_segment());
        
        // User segments (Ring 3)
        let user_data_selector = gdt.append(Descriptor::user_data_segment());
        let user_code_selector = gdt.append(Descriptor::user_code_segment());
        
        // Task State Segment
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
        
        (gdt, Selectors {
            kernel_code_selector,
            kernel_data_selector,
            user_code_selector,
            user_data_selector,
            tss_selector,
        })
    };
}

/// Segment selectors for accessing different privilege levels
pub struct Selectors {
    pub kernel_code_selector: SegmentSelector,
    pub kernel_data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
}

/// Initialize the GDT
pub fn init() {
    use x86_64::instructions::segmentation::{Segment, CS, DS, ES, SS};
    use x86_64::instructions::tables::load_tss;
    
    // Load the GDT
    GDT.0.load();
    
    unsafe {
        // Reload code segment register
        CS::set_reg(GDT.1.kernel_code_selector);
        
        // Reload data segment registers
        DS::set_reg(GDT.1.kernel_data_selector);
        ES::set_reg(GDT.1.kernel_data_selector);
        SS::set_reg(GDT.1.kernel_data_selector);
        
        // Load Task State Segment
        load_tss(GDT.1.tss_selector);
    }
}

/// Get the kernel code segment selector
pub fn kernel_code_selector() -> SegmentSelector {
    GDT.1.kernel_code_selector
}

/// Get the kernel data segment selector
pub fn kernel_data_selector() -> SegmentSelector {
    GDT.1.kernel_data_selector
}

/// Get the user code segment selector
pub fn user_code_selector() -> SegmentSelector {
    GDT.1.user_code_selector
}

/// Get the user data segment selector
pub fn user_data_selector() -> SegmentSelector {
    GDT.1.user_data_selector
}

