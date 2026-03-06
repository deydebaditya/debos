//! System Call Handlers
//!
//! Individual implementations for each system call.

use super::{SyscallResult, SyscallError};
use crate::scheduler::{self, ThreadId};
use crate::ipc::{self, EndpointId};

// ============================================================================
// Thread Management Syscalls
// ============================================================================

/// sys_thread_spawn(entry_point, stack_ptr, priority, capability_cptr) -> tid
pub fn sys_thread_spawn(
    entry_point: u64,
    _stack_ptr: u64,
    priority: u64,
    _capability: u64,
) -> SyscallResult {
    // Validate arguments
    if entry_point == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    if priority > 255 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // TODO: Validate capability
    
    let tid = scheduler::spawn_thread(entry_point as usize, priority as u8);
    Ok(tid.0)
}

/// sys_thread_yield()
pub fn sys_thread_yield() -> SyscallResult {
    scheduler::yield_now();
    Ok(0)
}

/// sys_thread_exit(exit_code)
pub fn sys_thread_exit(exit_code: i32) -> SyscallResult {
    scheduler::exit_thread(exit_code);
    // Never returns
}

/// sys_thread_get_id() -> tid
pub fn sys_thread_get_id() -> SyscallResult {
    scheduler::current_tid()
        .map(|tid| tid.0)
        .ok_or(SyscallError::NotFound)
}

// ============================================================================
// Memory Management Syscalls
// ============================================================================

/// Memory mapping flags
pub mod mem_flags {
    pub const READ: u64 = 0x01;
    pub const WRITE: u64 = 0x02;
    pub const EXECUTE: u64 = 0x04;
    pub const USER: u64 = 0x08;
}

/// sys_mem_map(phys_addr, virt_addr, size, flags) -> result
/// 
/// Maps physical memory to virtual address space.
/// 
/// Arguments:
/// - phys_addr: Physical address to map (must be page-aligned)
/// - virt_addr: Virtual address to map to (must be page-aligned)
/// - size: Size of mapping in bytes (will be rounded up to page size)
/// - flags: Protection flags (READ, WRITE, EXECUTE, USER)
///
/// Returns 0 on success, negative error code on failure.
pub fn sys_mem_map(
    phys_addr: u64,
    virt_addr: u64,
    size: u64,
    flags: u64,
) -> SyscallResult {
    const PAGE_SIZE: u64 = 4096;
    
    // Validate alignment
    if phys_addr % PAGE_SIZE != 0 || virt_addr % PAGE_SIZE != 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Validate size
    if size == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Round up size to page boundary
    let num_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;
    
    // Map each page
    for i in 0..num_pages {
        let paddr = phys_addr + i * PAGE_SIZE;
        let vaddr = virt_addr + i * PAGE_SIZE;
        
        #[cfg(target_arch = "x86_64")]
        {
            use crate::arch::x86_64::paging;
            
            let mut page_flags = paging::PageFlags::PRESENT;
            if flags & mem_flags::WRITE != 0 {
                page_flags |= paging::PageFlags::WRITABLE;
            }
            if flags & mem_flags::USER != 0 {
                page_flags |= paging::PageFlags::USER_ACCESSIBLE;
            }
            // Note: NX bit handling would go here for EXECUTE flag
            
            paging::map_page(vaddr as usize, paddr as usize, page_flags)
                .map_err(|_| SyscallError::OutOfMemory)?;
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            use crate::arch::aarch64::mmu;
            
            let mut attrs = mmu::PageAttributes::VALID | mmu::PageAttributes::AF;
            if flags & mem_flags::WRITE != 0 {
                // Writable
            } else {
                attrs |= mmu::PageAttributes::AP_RO;
            }
            if flags & mem_flags::USER != 0 {
                attrs |= mmu::PageAttributes::AP_USER;
            }
            if flags & mem_flags::EXECUTE == 0 {
                attrs |= mmu::PageAttributes::UXN | mmu::PageAttributes::PXN;
            }
            
            mmu::map_page(vaddr as usize, paddr as usize, attrs)
                .map_err(|_| SyscallError::OutOfMemory)?;
        }
    }
    
    Ok(0)
}

/// sys_mem_unmap(virt_addr, length) -> result
/// 
/// Unmaps virtual memory pages.
pub fn sys_mem_unmap(virt_addr: u64, length: u64) -> SyscallResult {
    const PAGE_SIZE: u64 = 4096;
    
    // Validate alignment
    if virt_addr % PAGE_SIZE != 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    if length == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    let num_pages = (length + PAGE_SIZE - 1) / PAGE_SIZE;
    
    for i in 0..num_pages {
        let vaddr = virt_addr + i * PAGE_SIZE;
        
        #[cfg(target_arch = "x86_64")]
        {
            use crate::arch::x86_64::paging;
            paging::unmap_page(vaddr as usize);
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            use crate::arch::aarch64::mmu;
            mmu::unmap_page(vaddr as usize);
        }
    }
    
    Ok(0)
}

/// sys_mem_protect(virt_addr, length, flags) -> result
/// 
/// Changes protection flags for memory pages.
pub fn sys_mem_protect(virt_addr: u64, length: u64, flags: u64) -> SyscallResult {
    const PAGE_SIZE: u64 = 4096;
    
    // Validate alignment
    if virt_addr % PAGE_SIZE != 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    if length == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // For now, we unmap and remap with new flags
    // A proper implementation would modify page table entries in place
    let num_pages = (length + PAGE_SIZE - 1) / PAGE_SIZE;
    
    for i in 0..num_pages {
        let vaddr = virt_addr + i * PAGE_SIZE;
        
        #[cfg(target_arch = "x86_64")]
        {
            use crate::arch::x86_64::paging;
            
            // Get current physical address
            if let Some(paddr) = paging::get_physical_address(vaddr as usize) {
                // Unmap and remap with new flags
                paging::unmap_page(vaddr as usize);
                
                let mut page_flags = paging::PageFlags::PRESENT;
                if flags & mem_flags::WRITE != 0 {
                    page_flags |= paging::PageFlags::WRITABLE;
                }
                if flags & mem_flags::USER != 0 {
                    page_flags |= paging::PageFlags::USER_ACCESSIBLE;
                }
                
                paging::map_page(vaddr as usize, paddr, page_flags)
                    .map_err(|_| SyscallError::OutOfMemory)?;
            }
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            // AArch64 implementation would go here
            // For now, just return success
            let _ = vaddr;
            let _ = flags;
        }
    }
    
    Ok(0)
}

// ============================================================================
// IPC Syscalls
// ============================================================================

/// sys_ipc_call(endpoint_cap, msg_ptr, len, reply_buf_ptr, reply_len)
pub fn sys_ipc_call(
    endpoint: u64,
    msg_ptr: u64,
    len: u64,
    reply_ptr: u64,
    reply_len: u64,
) -> SyscallResult {
    // Validate pointers
    if msg_ptr == 0 || reply_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    // Read message from user space
    let msg = unsafe {
        core::slice::from_raw_parts(msg_ptr as *const u8, len as usize)
    };
    
    // Prepare reply buffer
    let reply_buf = unsafe {
        core::slice::from_raw_parts_mut(reply_ptr as *mut u8, reply_len as usize)
    };
    
    // Make the IPC call
    let result = ipc::ipc_call(EndpointId(endpoint), msg, reply_buf)
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    Ok(result as u64)
}

/// sys_ipc_wait(endpoint_cap, buffer_ptr, buffer_len)
pub fn sys_ipc_wait(endpoint: u64, buffer_ptr: u64, buffer_len: u64) -> SyscallResult {
    if buffer_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    let buffer = unsafe {
        core::slice::from_raw_parts_mut(buffer_ptr as *mut u8, buffer_len as usize)
    };
    
    let (len, _sender) = ipc::ipc_wait(EndpointId(endpoint), buffer)
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    Ok(len as u64)
}

/// sys_ipc_reply(endpoint, caller_tid, reply_ptr, reply_len)
pub fn sys_ipc_reply(
    endpoint: u64,
    caller: u64,
    reply_ptr: u64,
    reply_len: u64,
) -> SyscallResult {
    if reply_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    let reply = unsafe {
        core::slice::from_raw_parts(reply_ptr as *const u8, reply_len as usize)
    };
    
    ipc::ipc_reply(EndpointId(endpoint), ThreadId(caller), reply)
        .map_err(|_| SyscallError::InvalidArgument)?;
    
    Ok(0)
}

/// sys_endpoint_create() -> endpoint_id
pub fn sys_endpoint_create() -> SyscallResult {
    let id = ipc::create_endpoint();
    Ok(id.0)
}

/// sys_endpoint_destroy(endpoint)
pub fn sys_endpoint_destroy(endpoint: u64) -> SyscallResult {
    ipc::destroy_endpoint(EndpointId(endpoint));
    Ok(0)
}

// ============================================================================
// Interrupt Syscalls
// ============================================================================

/// sys_irq_ack(irq_number)
#[cfg(target_arch = "x86_64")]
pub fn sys_irq_ack(irq: u8) -> SyscallResult {
    use crate::arch::x86_64::idt::PICS;
    
    unsafe {
        PICS.lock().notify_end_of_interrupt(irq + 32);
    }
    
    Ok(0)
}

#[cfg(target_arch = "aarch64")]
pub fn sys_irq_ack(irq: u8) -> SyscallResult {
    use crate::arch::aarch64::gic::GIC;
    
    GIC.lock().end_interrupt(irq as u32);
    
    Ok(0)
}

/// sys_irq_wait(irq_number)
pub fn sys_irq_wait(_irq: u8) -> SyscallResult {
    // TODO: Block until IRQ fires
    // For now, just return immediately
    Ok(0)
}

// ============================================================================
// Capability Syscalls
// ============================================================================

/// sys_cap_grant(source_cptr, target_slot, rights_mask)
pub fn sys_cap_grant(_source: u64, _target: u64, _rights: u64) -> SyscallResult {
    // TODO: Implement capability granting
    Ok(0)
}

/// sys_cap_revoke(cptr, target_thread)
pub fn sys_cap_revoke(_cptr: u64, _target: u64) -> SyscallResult {
    // TODO: Implement capability revocation
    Ok(0)
}

// ============================================================================
// Debug Syscalls
// ============================================================================

/// sys_debug_print(string_ptr, len)
pub fn sys_debug_print(ptr: u64, len: u64) -> SyscallResult {
    if ptr == 0 {
        return Err(SyscallError::BadAddress);
    }
    
    let bytes = unsafe {
        core::slice::from_raw_parts(ptr as *const u8, len as usize)
    };
    
    if let Ok(s) = core::str::from_utf8(bytes) {
        crate::print!("{}", s);
        Ok(len)
    } else {
        Err(SyscallError::InvalidArgument)
    }
}
