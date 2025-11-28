//! 4-Level Paging for x86_64
//!
//! Implements page table management for virtual memory:
//! - PML4 (Page Map Level 4) -> PDPT -> PD -> PT
//! - Each level has 512 entries, each entry is 8 bytes
//! - Total addressable: 48 bits virtual, 52 bits physical

use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags,
        PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};
use bootloader_api::info::{MemoryRegionKind, MemoryRegions};

/// Physical memory offset (set by bootloader)
static mut PHYSICAL_MEMORY_OFFSET: Option<VirtAddr> = None;

/// Page table mapper
static mut MAPPER: Option<OffsetPageTable<'static>> = None;

/// Frame allocator for physical memory
static mut FRAME_ALLOCATOR: Option<BootInfoFrameAllocator> = None;

/// Initialize the paging subsystem
pub unsafe fn init(physical_memory_offset: VirtAddr, memory_regions: &'static MemoryRegions) {
    PHYSICAL_MEMORY_OFFSET = Some(physical_memory_offset);
    
    // Get the level 4 page table
    let level_4_table = active_level_4_table(physical_memory_offset);
    
    // Create the mapper
    MAPPER = Some(OffsetPageTable::new(level_4_table, physical_memory_offset));
    
    // Create frame allocator
    FRAME_ALLOCATOR = Some(BootInfoFrameAllocator::init(memory_regions));
}

/// Returns a mutable reference to the active level 4 table
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;
    
    let (level_4_table_frame, _) = Cr3::read();
    
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    
    &mut *page_table_ptr
}

/// Map a virtual page to a physical frame
pub fn map_page(page: Page<Size4KiB>, frame: PhysFrame<Size4KiB>, flags: PageTableFlags) -> Result<(), &'static str> {
    unsafe {
        let mapper = MAPPER.as_mut().ok_or("Paging not initialized")?;
        let frame_allocator = FRAME_ALLOCATOR.as_mut().ok_or("Frame allocator not initialized")?;
        
        mapper
            .map_to(page, frame, flags, frame_allocator)
            .map_err(|_| "Failed to map page")?
            .flush();
        
        Ok(())
    }
}

/// Allocate a new physical frame
pub fn allocate_frame() -> Option<PhysFrame<Size4KiB>> {
    unsafe {
        FRAME_ALLOCATOR.as_mut()?.allocate_frame()
    }
}

/// Translate a virtual address to a physical address
pub fn translate_addr(addr: VirtAddr) -> Option<PhysAddr> {
    use x86_64::structures::paging::Translate;
    
    unsafe {
        MAPPER.as_ref()?.translate_addr(addr)
    }
}

/// Create a new empty page table for a user process
pub fn create_user_page_table() -> Option<PhysFrame<Size4KiB>> {
    let frame = allocate_frame()?;
    
    unsafe {
        let phys_mem_offset = PHYSICAL_MEMORY_OFFSET?;
        let virt = phys_mem_offset + frame.start_address().as_u64();
        let page_table: *mut PageTable = virt.as_mut_ptr();
        
        // Zero the page table
        core::ptr::write_bytes(page_table, 0, 1);
        
        // Copy kernel mappings (upper half of address space)
        let level_4_table = active_level_4_table(phys_mem_offset);
        for i in 256..512 {
            (&mut *page_table)[i] = level_4_table[i].clone();
        }
    }
    
    Some(frame)
}

/// Frame allocator that uses the bootloader's memory map
pub struct BootInfoFrameAllocator {
    memory_regions: &'static MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a new frame allocator from the bootloader's memory map
    pub unsafe fn init(memory_regions: &'static MemoryRegions) -> Self {
        BootInfoFrameAllocator {
            memory_regions,
            next: 0,
        }
    }
    
    /// Returns an iterator over usable frames
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        // Get usable regions from memory map
        let regions = self.memory_regions.iter();
        let usable_regions = regions.filter(|r| r.kind == MemoryRegionKind::Usable);
        
        // Map each region to its address range
        let addr_ranges = usable_regions.map(|r| r.start..r.end);
        
        // Transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        
        // Create PhysFrame types from start addresses
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

/// Page flags for different mapping types
pub mod flags {
    use x86_64::structures::paging::PageTableFlags;
    
    /// Kernel code (read, execute)
    pub const KERNEL_CODE: PageTableFlags = PageTableFlags::PRESENT;
    
    /// Kernel data (read, write)
    pub const KERNEL_DATA: PageTableFlags = PageTableFlags::from_bits_truncate(
        PageTableFlags::PRESENT.bits() | PageTableFlags::WRITABLE.bits()
    );
    
    /// User code (read, execute, user accessible)
    pub const USER_CODE: PageTableFlags = PageTableFlags::from_bits_truncate(
        PageTableFlags::PRESENT.bits() | PageTableFlags::USER_ACCESSIBLE.bits()
    );
    
    /// User data (read, write, user accessible)
    pub const USER_DATA: PageTableFlags = PageTableFlags::from_bits_truncate(
        PageTableFlags::PRESENT.bits() | 
        PageTableFlags::WRITABLE.bits() | 
        PageTableFlags::USER_ACCESSIBLE.bits()
    );
}

