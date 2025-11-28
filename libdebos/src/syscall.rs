//! System Call Wrappers
//!
//! Raw syscall interface for user-space programs.

use core::arch::asm;

/// Perform a system call with up to 6 arguments
#[inline(always)]
pub unsafe fn syscall(
    num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> i64 {
    let result: i64;
    
    asm!(
        "syscall",
        inlateout("rax") num => result,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        in("r9") arg6,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack, preserves_flags)
    );
    
    result
}

// Syscall numbers
pub const SYS_THREAD_SPAWN: u64 = 1;
pub const SYS_THREAD_YIELD: u64 = 2;
pub const SYS_THREAD_EXIT: u64 = 3;
pub const SYS_THREAD_GET_ID: u64 = 4;
pub const SYS_MEM_MAP: u64 = 10;
pub const SYS_IPC_CALL: u64 = 20;
pub const SYS_IPC_WAIT: u64 = 21;
pub const SYS_IPC_REPLY: u64 = 22;
pub const SYS_DEBUG_PRINT: u64 = 100;

