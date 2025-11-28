//! Syscall Dispatcher
//!
//! Sets up the system call mechanism and dispatches to the appropriate handler.

#[cfg(target_arch = "x86_64")]
mod x86_64_impl {
    use x86_64::registers::model_specific::{Efer, EferFlags, LStar, Star, SFMask};
    use x86_64::registers::rflags::RFlags;
    use x86_64::VirtAddr;
    use core::arch::naked_asm;
    
    use super::super::{SyscallNumber, SyscallResult, SyscallError};
    use super::super::handlers;
    
    /// Initialize the syscall MSRs for x86_64
    pub fn init() {
        unsafe {
            // Enable syscall/sysret instructions
            Efer::update(|flags| *flags |= EferFlags::SYSTEM_CALL_EXTENSIONS);
            
            // Set up segment selectors
            Star::write(
                x86_64::structures::gdt::SegmentSelector(0x23), // User code
                x86_64::structures::gdt::SegmentSelector(0x1B), // User data
                x86_64::structures::gdt::SegmentSelector(0x08), // Kernel code
                x86_64::structures::gdt::SegmentSelector(0x10), // Kernel data
            ).unwrap();
            
            // Set syscall entry point
            LStar::write(VirtAddr::new(syscall_entry as *const () as u64));
            
            // Clear interrupt flag on syscall entry
            SFMask::write(RFlags::INTERRUPT_FLAG);
        }
    }
    
    /// Low-level syscall entry point
    #[unsafe(naked)]
    unsafe extern "C" fn syscall_entry() {
        naked_asm!(
            "swapgs",
            "push rcx",
            "push r11",
            "push rbp",
            "push rbx",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            "mov rcx, r10",
            "call {dispatcher}",
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbx",
            "pop rbp",
            "pop r11",
            "pop rcx",
            "swapgs",
            "sysretq",
            dispatcher = sym syscall_dispatch,
        );
    }
    
    extern "C" fn syscall_dispatch(
        syscall_num: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        arg5: u64,
        _arg6: u64,
    ) -> i64 {
        let result = match SyscallNumber::try_from(syscall_num) {
            Ok(num) => super::dispatch_syscall(num, arg1, arg2, arg3, arg4, arg5),
            Err(_) => Err(SyscallError::InvalidSyscall),
        };
        
        match result {
            Ok(val) => val as i64,
            Err(err) => err as i64,
        }
    }
}

#[cfg(target_arch = "aarch64")]
mod aarch64_impl {
    /// Initialize syscall handling for AArch64
    /// Note: On AArch64, system calls use SVC instruction and are handled
    /// through the exception vector (sync_exception handler)
    pub fn init() {
        // SVC handling is set up in the exception vectors
        // No additional initialization needed here
    }
}

use super::{SyscallNumber, SyscallResult, SyscallError};
use super::handlers;

/// Initialize the syscall dispatcher
pub fn init() {
    #[cfg(target_arch = "x86_64")]
    x86_64_impl::init();
    
    #[cfg(target_arch = "aarch64")]
    aarch64_impl::init();
}

/// Dispatch to the appropriate syscall handler (shared between architectures)
pub fn dispatch_syscall(
    num: SyscallNumber,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
) -> SyscallResult {
    match num {
        // Thread management
        SyscallNumber::ThreadSpawn => handlers::sys_thread_spawn(arg1, arg2, arg3, arg4),
        SyscallNumber::ThreadYield => handlers::sys_thread_yield(),
        SyscallNumber::ThreadExit => handlers::sys_thread_exit(arg1 as i32),
        SyscallNumber::ThreadGetId => handlers::sys_thread_get_id(),
        
        // Memory management
        SyscallNumber::MemMap => handlers::sys_mem_map(arg1, arg2, arg3, arg4),
        SyscallNumber::MemUnmap => handlers::sys_mem_unmap(arg1, arg2),
        SyscallNumber::MemProtect => handlers::sys_mem_protect(arg1, arg2, arg3),
        
        // IPC
        SyscallNumber::IpcCall => handlers::sys_ipc_call(arg1, arg2, arg3, arg4, arg5),
        SyscallNumber::IpcWait => handlers::sys_ipc_wait(arg1, arg2, arg3),
        SyscallNumber::IpcReply => handlers::sys_ipc_reply(arg1, arg2, arg3, arg4),
        SyscallNumber::EndpointCreate => handlers::sys_endpoint_create(),
        SyscallNumber::EndpointDestroy => handlers::sys_endpoint_destroy(arg1),
        
        // Interrupts
        SyscallNumber::IrqAck => handlers::sys_irq_ack(arg1 as u8),
        SyscallNumber::IrqWait => handlers::sys_irq_wait(arg1 as u8),
        
        // Capabilities
        SyscallNumber::CapGrant => handlers::sys_cap_grant(arg1, arg2, arg3),
        SyscallNumber::CapRevoke => handlers::sys_cap_revoke(arg1, arg2),
        
        // Debug
        SyscallNumber::DebugPrint => handlers::sys_debug_print(arg1, arg2),
    }
}
