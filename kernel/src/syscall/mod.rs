//! System Call Interface
//!
//! Implements the syscall instruction handler and dispatches to
//! individual syscall implementations.
//!
//! ## Syscall ABI (x86_64 System V):
//! - RAX: syscall number
//! - RDI, RSI, RDX, R10, R8, R9: arguments 1-6
//! - RAX: return value
//! - RCX, R11: clobbered by syscall instruction

pub mod dispatcher;
pub mod handlers;

/// System call numbers
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallNumber {
    // Thread management
    ThreadSpawn = 1,
    ThreadYield = 2,
    ThreadExit = 3,
    ThreadGetId = 4,
    
    // Memory management
    MemMap = 10,
    MemUnmap = 11,
    MemProtect = 12,
    
    // IPC
    IpcCall = 20,
    IpcWait = 21,
    IpcReply = 22,
    EndpointCreate = 23,
    EndpointDestroy = 24,
    
    // Interrupts
    IrqAck = 30,
    IrqWait = 31,
    
    // Capabilities
    CapGrant = 40,
    CapRevoke = 41,
    
    // Debug/console
    DebugPrint = 100,
}

impl TryFrom<u64> for SyscallNumber {
    type Error = ();
    
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(SyscallNumber::ThreadSpawn),
            2 => Ok(SyscallNumber::ThreadYield),
            3 => Ok(SyscallNumber::ThreadExit),
            4 => Ok(SyscallNumber::ThreadGetId),
            10 => Ok(SyscallNumber::MemMap),
            11 => Ok(SyscallNumber::MemUnmap),
            12 => Ok(SyscallNumber::MemProtect),
            20 => Ok(SyscallNumber::IpcCall),
            21 => Ok(SyscallNumber::IpcWait),
            22 => Ok(SyscallNumber::IpcReply),
            23 => Ok(SyscallNumber::EndpointCreate),
            24 => Ok(SyscallNumber::EndpointDestroy),
            30 => Ok(SyscallNumber::IrqAck),
            31 => Ok(SyscallNumber::IrqWait),
            40 => Ok(SyscallNumber::CapGrant),
            41 => Ok(SyscallNumber::CapRevoke),
            100 => Ok(SyscallNumber::DebugPrint),
            _ => Err(()),
        }
    }
}

/// System call result type
pub type SyscallResult = Result<u64, SyscallError>;

/// System call error codes
#[repr(i64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    /// Invalid syscall number
    InvalidSyscall = -1,
    /// Invalid argument
    InvalidArgument = -2,
    /// Permission denied
    PermissionDenied = -3,
    /// Resource not found
    NotFound = -4,
    /// Resource busy
    Busy = -5,
    /// Out of memory
    OutOfMemory = -6,
    /// Invalid capability
    InvalidCapability = -7,
    /// Operation would block
    WouldBlock = -8,
    /// Interrupted
    Interrupted = -9,
    /// Invalid address
    BadAddress = -10,
}

impl From<SyscallError> for i64 {
    fn from(err: SyscallError) -> i64 {
        err as i64
    }
}

/// Initialize the syscall interface
pub fn init() {
    dispatcher::init();
}

