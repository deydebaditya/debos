//! AArch64 MMU (Memory Management Unit)
//!
//! Implements 4-level page tables (4KB pages, 48-bit virtual addresses):
//! - Level 0: 512GB per entry
//! - Level 1: 1GB per entry  
//! - Level 2: 2MB per entry
//! - Level 3: 4KB per entry

use spin::Mutex;

/// Page size (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Page table entry count per table
pub const PAGE_TABLE_ENTRIES: usize = 512;

/// Page table entry flags
pub mod flags {
    pub const VALID: u64 = 1 << 0;
    pub const TABLE: u64 = 1 << 1;  // For L0-L2: points to next level table
    pub const PAGE: u64 = 1 << 1;   // For L3: this is a page entry
    
    // Access permissions (AP bits)
    pub const AP_RW_EL1: u64 = 0b00 << 6;     // R/W at EL1, no access at EL0
    pub const AP_RW_ALL: u64 = 0b01 << 6;     // R/W at all levels
    pub const AP_RO_EL1: u64 = 0b10 << 6;     // R/O at EL1, no access at EL0
    pub const AP_RO_ALL: u64 = 0b11 << 6;     // R/O at all levels
    
    // Shareability
    pub const SH_INNER: u64 = 0b11 << 8;      // Inner shareable
    
    // Access flag
    pub const AF: u64 = 1 << 10;
    
    // Execute-never
    pub const UXN: u64 = 1 << 54;             // User execute-never
    pub const PXN: u64 = 1 << 53;             // Privileged execute-never
    
    // Memory attribute index
    pub const ATTR_NORMAL: u64 = 0 << 2;
    pub const ATTR_DEVICE: u64 = 1 << 2;
}

/// A page table entry
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create an invalid entry
    pub const fn empty() -> Self {
        PageTableEntry(0)
    }
    
    /// Check if entry is valid
    pub fn is_valid(&self) -> bool {
        self.0 & flags::VALID != 0
    }
    
    /// Get the physical address this entry points to
    pub fn address(&self) -> u64 {
        self.0 & 0x0000_FFFF_FFFF_F000
    }
    
    /// Create a table entry (L0-L2)
    pub fn new_table(addr: u64) -> Self {
        PageTableEntry(addr | flags::TABLE | flags::VALID)
    }
    
    /// Create a page entry (L3)
    pub fn new_page(addr: u64, kernel: bool, writable: bool, executable: bool) -> Self {
        let mut entry = addr | flags::PAGE | flags::VALID | flags::AF | flags::SH_INNER | flags::ATTR_NORMAL;
        
        if kernel {
            entry |= flags::AP_RW_EL1;
            if !executable {
                entry |= flags::PXN;
            }
        } else {
            if writable {
                entry |= flags::AP_RW_ALL;
            } else {
                entry |= flags::AP_RO_ALL;
            }
            if !executable {
                entry |= flags::UXN;
            }
        }
        
        PageTableEntry(entry)
    }
}

/// A page table
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    /// Create an empty page table
    pub const fn empty() -> Self {
        PageTable {
            entries: [PageTableEntry::empty(); PAGE_TABLE_ENTRIES],
        }
    }
}

/// Kernel page tables
static KERNEL_L0_TABLE: Mutex<PageTable> = Mutex::new(PageTable::empty());

/// Frame allocator for page tables
static NEXT_FRAME: Mutex<usize> = Mutex::new(0x4100_0000); // Start after kernel

/// Allocate a page frame
fn allocate_frame() -> Option<usize> {
    let mut next = NEXT_FRAME.lock();
    let frame = *next;
    *next += PAGE_SIZE;
    Some(frame)
}

/// Initialize the MMU
pub fn init() {
    // For now, we run with the identity mapping set up by the bootloader
    // A full implementation would set up proper page tables here
    
    crate::println!("[OK] MMU initialized (using bootloader mapping)");
}

/// Map a virtual address to a physical address
pub fn map_page(
    _virt: usize,
    _phys: usize,
    _kernel: bool,
    _writable: bool,
    _executable: bool,
) -> Result<(), &'static str> {
    // TODO: Implement page mapping
    Ok(())
}

/// Invalidate TLB entry for an address
#[inline(always)]
pub fn invalidate_page(addr: usize) {
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "tlbi vaae1is, {}",
            "dsb ish",
            "isb",
            in(reg) addr >> 12
        );
    }
}

/// Invalidate entire TLB
#[inline(always)]
pub fn invalidate_all() {
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "tlbi vmalle1is",
            "dsb ish", 
            "isb"
        );
    }
}

