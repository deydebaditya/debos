# DebOS Beginner's Guide: Complete System Flow Explanation

> **For:** Complete beginners to both Rust and operating systems  
> **Goal:** Understand how DebOS works from boot to shell interaction

---

## Table of Contents

1. [What is DebOS?](#what-is-debos)
2. [System Architecture Overview](#system-architecture-overview)
3. [Flow 1: Boot Process](#flow-1-boot-process)
4. [Flow 2: Memory Initialization](#flow-2-memory-initialization)
5. [Flow 3: Scheduler and Threads](#flow-3-scheduler-and-threads)
6. [Flow 4: Shell Interaction](#flow-4-shell-interaction)
7. [Flow 5: Command Execution](#flow-5-command-execution)
8. [Flow 6: Filesystem Operations](#flow-6-filesystem-operations)
9. [Flow 7: System Calls](#flow-7-system-calls)
10. [Key Rust Concepts Used](#key-rust-concepts-used)

---

## What is DebOS?

**DebOS** is an operating system (OS) - like Linux or Windows, but much simpler. It's written in **Rust**, a programming language that prevents many common bugs.

### Key Concepts for Beginners:

1. **Operating System**: Software that manages your computer's hardware (CPU, memory, disk, keyboard, etc.)
2. **Kernel**: The core part of the OS that runs with special privileges
3. **Microkernel**: A design where the kernel does minimal work; most features run as separate programs
4. **Rust**: A programming language that's memory-safe (prevents crashes from memory bugs)

### What Makes DebOS Special?

- **Microkernel Design**: The kernel is tiny; most features run as separate programs
- **Written in Rust**: Memory-safe, so fewer crashes
- **Multi-Architecture**: Works on both Intel/AMD (x86_64) and Apple Silicon (AArch64)

---

## System Architecture Overview

Think of DebOS like a layered cake:

```
┌─────────────────────────────────────────┐
│  Applications (future)                   │  ← User programs
├─────────────────────────────────────────┤
│  Servers (VFS, Network, Device Manager)  │  ← OS services
├─────────────────────────────────────────┤
│  Kernel (DeK) - Scheduling, Memory, IPC  │  ← Core OS
└─────────────────────────────────────────┘
```

**The Kernel (DeK)** provides only:
- **Scheduling**: Decides which program runs when
- **Memory Management**: Allocates memory to programs
- **IPC**: Lets programs talk to each other
- **System Calls**: Interface for programs to request OS services

Everything else (filesystem, networking, device drivers) runs as separate programs!

---

## Flow 1: Boot Process

**Question:** How does the computer start running DebOS?

### Step-by-Step Boot Flow:

#### 1.1 Power On → Assembly Code (`kernel/src/main.rs`)

When you run `make run-arm`, QEMU (a virtual machine) starts and loads the DebOS kernel. The very first code that runs is **assembly language** (low-level instructions for the CPU).

**File:** `kernel/src/main.rs` (lines 23-64)

```rust
// This is assembly code embedded in Rust
global_asm!(
    r#"
_start:
    // Check: Are we on CPU core 0? (Only core 0 boots)
    mrs     x1, mpidr_el1
    and     x1, x1, #3
    cbz     x1, 2f
    
    // Other cores: wait forever (park them)
1:  wfe
    b       1b

2:  // Core 0 continues here
    // Enable floating-point unit
    mov     x0, #(3 << 20)
    msr     CPACR_EL1, x0
    
    // Set up stack pointer (where function calls store data)
    ldr     x1, =__stack_top
    mov     sp, x1
    
    // Clear BSS (uninitialized data section - set to zero)
    ldr     x1, =__bss_start
    ldr     x2, =__bss_end
3:  cmp     x1, x2
    b.ge    4f
    str     xzr, [x1], #8
    b       3b
    
4:  // Jump to Rust code!
    bl      kernel_main_aarch64
"#
);
```

**What's happening:**
- **Line 30-32**: Check if we're on CPU core 0. If not, park the core (wait forever).
- **Line 47-48**: Set up the **stack** - a memory area for function calls.
- **Line 51-56**: Clear **BSS** - memory that should start as zero.
- **Line 59**: Jump to Rust code (`kernel_main_aarch64`).

**Why assembly first?** Rust needs a stack and cleared memory before it can run. Assembly sets this up.

---

#### 1.2 Rust Entry Point (`kernel/src/main.rs`)

**File:** `kernel/src/main.rs` (lines 70-97)

```rust
pub extern "C" fn kernel_main_aarch64() -> ! {
    // 1. Initialize UART (serial port for printing)
    debos_kernel::arch::aarch64::uart::init();
    
    // 2. Print welcome banner
    debos_kernel::println!("╔═══════════════════════════════════════════════════════════════╗");
    debos_kernel::println!("║                DebOS Nano-Kernel (AArch64)                    ║");
    
    // 3. Initialize exception handling (for crashes/errors)
    debos_kernel::arch::aarch64::exceptions::init();
    
    // 4. Initialize GIC (interrupt controller - handles hardware events)
    debos_kernel::arch::aarch64::gic::init();
    
    // 5. Initialize memory management
    debos_kernel::memory::init_aarch64();
    
    // 6. Continue with common initialization
    debos_kernel::kernel_init()
}
```

**What's happening:**
1. **UART Init**: Sets up serial port so we can print messages
2. **Exceptions**: Sets up handlers for crashes/errors
3. **GIC**: Sets up interrupt controller (handles timer, keyboard, etc.)
4. **Memory**: Initializes memory management (allocator, page tables)
5. **kernel_init()**: Continues with the rest of initialization

**Key Rust concept:** `-> !` means "this function never returns" (it runs forever or crashes).

---

#### 1.3 Common Kernel Initialization (`kernel/src/lib.rs`)

**File:** `kernel/src/lib.rs` (lines 40-78)

```rust
pub fn kernel_init() -> ! {
    // 1. Initialize scheduler (decides which thread runs)
    scheduler::init();
    
    // 2. Initialize syscall interface (how programs request OS services)
    syscall::init();
    
    // 3. Initialize drivers (hardware support)
    drivers::init();
    
    // 4. Initialize filesystem
    fs::init();
    
    // 5. Initialize security (user management)
    security::init();
    
    // 6. Enable interrupts (allow timer, keyboard, etc. to interrupt CPU)
    #[cfg(target_arch = "aarch64")]
    arch::aarch64::gic::enable_timer();
    arch::enable_interrupts();
    
    // 7. Start the shell
    start_shell();
    
    // 8. Enter idle loop (CPU sleeps until interrupted)
    idle_loop()
}
```

**What's happening:**
1. **Scheduler**: Sets up the thread scheduler (we'll explain this later)
2. **Syscalls**: Sets up system call interface
3. **Drivers**: Initializes device drivers
4. **Filesystem**: Sets up filesystem support
5. **Security**: Initializes user management
6. **Interrupts**: Enables interrupts (timer, keyboard, etc.)
7. **Shell**: Starts the interactive shell
8. **Idle Loop**: CPU sleeps, waiting for interrupts

**Flow so far:**
```
Power On → Assembly Setup → Rust Entry → Initialize Everything → Start Shell → Idle Loop
```

---

## Flow 2: Memory Initialization

**Question:** How does DebOS manage memory?

### 2.1 What is Memory Management?

Your computer has **RAM** (Random Access Memory) - temporary storage for programs. The OS must:
- Track which memory is used/free
- Allocate memory to programs
- Prevent programs from accessing each other's memory

### 2.2 Memory Initialization Flow

**File:** `kernel/src/memory/mod.rs` (simplified)

```rust
pub fn init_aarch64() {
    // 1. Set up page tables (maps virtual addresses to physical addresses)
    // 2. Initialize buddy allocator (manages free memory blocks)
    // 3. Initialize heap allocator (for dynamic memory allocation)
}
```

**What's a page table?**
- Programs use **virtual addresses** (like "memory address 0x1000")
- The CPU translates these to **physical addresses** (actual RAM location)
- **Page tables** store this translation

**What's a buddy allocator?**
- Manages free memory in blocks of power-of-2 sizes (4KB, 8KB, 16KB, etc.)
- When you need 5KB, it gives you 8KB (next power of 2)
- When you free memory, it merges adjacent free blocks

**What's a heap allocator?**
- Provides `alloc()` and `dealloc()` functions
- Used by Rust's `Vec`, `String`, `Box`, etc.

### 2.3 Memory Layout

```
┌─────────────────────────────────────┐
│ 0x0000_0000                         │
│  Kernel Code (Text)                 │  ← Executable code
├─────────────────────────────────────┤
│  Kernel Data (Read-only)             │  ← Constants
├─────────────────────────────────────┤
│  Kernel Data (Read-write)            │  ← Variables
├─────────────────────────────────────┤
│  BSS (Zero-initialized)              │  ← Uninitialized variables
├─────────────────────────────────────┤
│  Heap (Dynamic allocation)          │  ← Grows upward
├─────────────────────────────────────┤
│  ...                                 │
├─────────────────────────────────────┤
│  Stack (Function calls)              │  ← Grows downward
│  ...                                 │
└─────────────────────────────────────┘
```

---

## Flow 3: Scheduler and Threads

**Question:** How does DebOS run multiple programs (threads) at the same time?

### 3.1 What is a Thread?

A **thread** is a sequence of instructions that can run independently. Think of it like a worker:
- Each thread has its own **stack** (memory for function calls)
- Each thread has a **program counter** (where it is in the code)
- The **scheduler** decides which thread runs when

### 3.2 Scheduler Overview

**File:** `kernel/src/scheduler/mod.rs`

The scheduler uses a **priority queue**:
- **High priority threads** (0-31): Run first, can preempt lower priority
- **Normal priority threads** (32-255): Run in round-robin fashion

### 3.3 Thread Lifecycle

```
┌──────────┐
│  Ready   │  ← Thread is ready to run
└────┬─────┘
     │ Scheduler picks it
     ▼
┌──────────┐
│ Running  │  ← Thread is currently executing
└────┬─────┘
     │ Blocks (waits for I/O) or preempted (timer interrupt)
     ▼
┌──────────┐
│ Blocked  │  ← Thread waiting for something
└────┬─────┘
     │ Event occurs (I/O ready)
     ▼
┌──────────┐
│  Ready   │  ← Back to ready queue
└──────────┘
```

### 3.4 Context Switching

**What is context switching?**

When the scheduler switches from Thread A to Thread B:
1. **Save Thread A's state**: CPU registers, stack pointer, program counter
2. **Load Thread B's state**: Restore its registers, stack, program counter
3. **Resume Thread B**: Continue from where it left off

**File:** `kernel/src/arch/aarch64/context.rs`

```rust
// This is assembly code that saves/restores CPU state
pub unsafe extern "C" fn context_switch(
    old_ctx: *mut ArchContext,  // Where to save current thread's state
    new_ctx: *const ArchContext  // Where to load new thread's state
) {
    // Save all CPU registers (x0-x30, SP, PC, etc.)
    // Restore new thread's registers
    // Jump to new thread's program counter
}
```

**What's in `ArchContext`?**
- All CPU registers (x0-x30 on ARM, rax-rdi on x86)
- Stack pointer (SP)
- Program counter (PC) - where to resume execution
- Status flags

### 3.5 Timer Interrupts and Preemption

**How does the scheduler get control?**

A **timer interrupt** fires every few milliseconds:
1. CPU stops current thread
2. Jumps to interrupt handler
3. Handler calls scheduler
4. Scheduler picks next thread
5. Context switch to new thread

**File:** `kernel/src/scheduler/mod.rs` (simplified)

```rust
pub fn on_timer_tick() {
    // Increment tick counter
    TICKS.fetch_add(1, Ordering::Relaxed);
    
    // Check if we should preempt current thread
    if should_preempt() {
        schedule();  // Switch to next thread
    }
}
```

**Flow:**
```
Timer Interrupt → Interrupt Handler → Scheduler → Context Switch → New Thread Runs
```

---

## Flow 4: Shell Interaction

**Question:** How does the shell read your keyboard input and display output?

### 4.1 Shell Startup

**File:** `kernel/src/lib.rs` (lines 81-92)

```rust
fn start_shell() {
    // Spawn shell as a high-priority kernel thread
    let shell_tid = scheduler::spawn_thread(
        shell::shell_thread_entry as *const () as usize, 
        64  // High priority
    );
    
    // Start scheduler to run shell immediately
    scheduler::start_scheduler();
}
```

**What's happening:**
1. **spawn_thread()**: Creates a new thread for the shell
2. **shell_thread_entry**: The function the shell thread will run
3. **Priority 64**: High priority so shell is responsive
4. **start_scheduler()**: Starts the scheduler, which runs the shell thread

### 4.2 Shell Main Loop

**File:** `kernel/src/shell/mod.rs` (lines 49-80)

```rust
pub fn run(&mut self) {
    self.print_banner();  // Print welcome message
    
    while self.running {
        self.print_prompt();  // Print "debos (/)> "
        
        // Read a line of input
        match self.read_line() {
            Some(line) => {
                if !line.trim().is_empty() {
                    self.execute(line.trim());  // Run the command
                }
            }
            None => {
                // Ctrl+C or Ctrl+D pressed
                continue;
            }
        }
    }
}
```

**Flow:**
```
Print Prompt → Read Input → Parse Command → Execute → Print Result → Repeat
```

### 4.3 Reading Input

**File:** `kernel/src/shell/input.rs`

```rust
pub fn read_char() -> Option<u8> {
    #[cfg(target_arch = "aarch64")]
    {
        use crate::arch::aarch64::uart::UART;
        UART.lock().read_byte()  // Read from UART (serial port)
    }
}
```

**What's a UART?**
- **UART** = Universal Asynchronous Receiver-Transmitter
- It's a serial port (like old modems used)
- QEMU connects it to your terminal
- When you type, characters go: **Keyboard → Terminal → QEMU → UART → Kernel**

**File:** `kernel/src/arch/aarch64/uart.rs`

```rust
pub fn read_byte(&mut self) -> Option<u8> {
    unsafe {
        let base = self.base as *mut u32;
        let fr = base.add(regs::FR / 4).read_volatile();
        
        // Check if receive FIFO is empty
        if (fr & flags::RXFE) != 0 {
            return None;  // No data available
        }
        
        // Read byte from data register
        let data = base.add(regs::DR / 4).read_volatile() as u8;
        Some(data)
    }
}
```

**What's happening:**
1. **Read Flag Register**: Check if data is available
2. **If empty**: Return `None` (no character)
3. **If data available**: Read byte from Data Register, return it

**Flow for reading a line:**
```
read_line() → Loop: read_char() → Check for Enter/Backspace/Ctrl+C → Build string → Return
```

### 4.4 Polling vs Interrupts

**Current Implementation:** **Polling**
- Shell continuously checks UART: "Is there a character? No? Check again."
- Uses CPU cycles even when no input

**Better Implementation (future):** **Interrupts**
- UART fires interrupt when character arrives
- CPU stops current work, handles character, resumes
- More efficient

**Why polling now?**
- Simpler to implement
- Works for basic shell
- Can be improved later

---

## Flow 5: Command Execution

**Question:** How does the shell execute commands like `mkdir`, `ls`, `whoami`?

### 5.1 Command Parsing

**File:** `kernel/src/shell/mod.rs` (lines 90-120, simplified)

```rust
fn execute(&mut self, command: &str) {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }
    
    let cmd = parts[0];  // First word is the command
    let args = &parts[1..];  // Rest are arguments
    
    match cmd {
        "mkdir" => commands::mkdir(args),
        "ls" => commands::ls(args),
        "whoami" => commands::whoami(args),
        // ... many more commands
        _ => println!("Unknown command: {}", cmd),
    }
}
```

**What's happening:**
1. **Split command**: "mkdir test" → `["mkdir", "test"]`
2. **Extract command**: `cmd = "mkdir"`
3. **Extract arguments**: `args = ["test"]`
4. **Match command**: Find handler function
5. **Call handler**: `commands::mkdir(["test"])`

### 5.2 Example: `mkdir` Command

**File:** `kernel/src/shell/commands.rs`

```rust
pub fn mkdir(args: &[&str]) {
    if args.is_empty() {
        println!("mkdir: missing operand");
        return;
    }
    
    let path = args[0];
    
    // Call filesystem function
    match crate::fs::mkdir(path) {
        Ok(()) => {
            // Success - no output (like Unix mkdir)
        }
        Err(e) => {
            println!("mkdir: cannot create directory '{}': {}", path, e);
        }
    }
}
```

**Flow:**
```
User types "mkdir test" → Shell parses → calls mkdir(["test"]) → fs::mkdir("test") → VFS creates directory
```

### 5.3 Example: `whoami` Command

**File:** `kernel/src/shell/commands.rs`

```rust
pub fn whoami(_args: &[&str]) {
    // Use SDK for safe credential access (avoids deadlocks)
    let username = crate::shell::sdk::current_username();
    println!("{}", username);
}
```

**What's the SDK?**
- **SDK** = Software Development Kit
- Provides safe functions for shell commands
- Avoids deadlocks (we'll explain this later)

**Flow:**
```
whoami → sdk::current_username() → security::database::get_user_by_uid() → Print username
```

---

## Flow 6: Filesystem Operations

**Question:** How does `mkdir`, `ls`, `cat` work with files and directories?

### 6.1 Filesystem Architecture

DebOS has multiple filesystem layers:

```
┌─────────────────────────────────────┐
│  Shell Commands (mkdir, ls, cat)   │  ← User interface
├─────────────────────────────────────┤
│  VFS (Virtual Filesystem)           │  ← Unified interface
├─────────────────────────────────────┤
│  Filesystem Drivers                 │  ← Actual implementation
│  - RamFS (in-memory)                │
│  - FAT32 (disk filesystem)          │
│  - ext4 (Linux filesystem)           │
└─────────────────────────────────────┘
```

**What's VFS?**
- **VFS** = Virtual Filesystem
- Provides a unified interface for all filesystems
- Shell commands call VFS, VFS calls the right driver

### 6.2 `mkdir` Flow

**File:** `kernel/src/fs/vfs.rs`

```rust
pub fn mkdir(path: &str) -> Result<(), FsError> {
    // 1. Parse path (e.g., "/test" or "test")
    let (parent, name) = parse_path(path)?;
    
    // 2. Get current user's UID/GID (for file ownership)
    let (uid, gid) = get_current_owner();
    
    // 3. Find parent directory
    let parent_node = find_node(&parent)?;
    
    // 4. Create new directory node
    let new_dir = create_directory(name, uid, gid)?;
    
    // 5. Add to parent
    parent_node.add_child(new_dir)?;
    
    Ok(())
}
```

**What's a node?**
- A **node** represents a file or directory
- Contains: name, type (file/dir), permissions, owner, children (if directory)

**Flow:**
```
mkdir("test") → Parse path → Find parent → Create node → Add to parent → Success
```

### 6.3 `ls` Flow

**File:** `kernel/src/shell/commands.rs`

```rust
pub fn ls(args: &[&str]) {
    let path = args.get(0).unwrap_or(&".");
    
    match crate::fs::readdir(path) {
        Ok(entries) => {
            for entry in entries {
                println!("{}", entry.name);
            }
        }
        Err(e) => {
            println!("ls: cannot access '{}': {}", path, e);
        }
    }
}
```

**Flow:**
```
ls(".") → fs::readdir(".") → VFS finds directory → Returns entries → Print each name
```

### 6.4 RamFS (In-Memory Filesystem)

**What is RamFS?**
- Files and directories stored in RAM (not on disk)
- Lost when system reboots
- Fast (no disk I/O)
- Used for temporary files, `/tmp`, etc.

**File:** `kernel/src/fs/ramfs.rs` (simplified)

```rust
pub struct RamFS {
    root: Node,  // Root directory
}

impl RamFS {
    fn create_directory(&mut self, name: &str) -> Node {
        Node {
            name: name.to_string(),
            node_type: NodeType::Directory,
            children: Vec::new(),
            // ... other fields
        }
    }
}
```

---

## Flow 7: System Calls

**Question:** How do programs request OS services (like reading files, creating threads)?

### 7.1 What is a System Call?

A **system call** (syscall) is how a program asks the OS to do something:
- **Create a thread**: `syscall(SPAWN_THREAD, ...)`
- **Read a file**: `syscall(READ, fd, buffer, size)`
- **Write to console**: `syscall(WRITE, fd, buffer, size)`

### 7.2 System Call Flow

```
User Program → syscall instruction → Kernel Handler → Execute → Return Result
```

**On AArch64:**
- Program executes `svc #0` (Supervisor Call)
- CPU jumps to exception handler
- Handler calls syscall dispatcher
- Dispatcher calls appropriate handler
- Result returned to program

**File:** `kernel/src/syscall/dispatcher.rs`

```rust
pub fn dispatch_syscall(
    num: SyscallNumber,  // Which syscall? (READ, WRITE, etc.)
    arg1: u64,
    arg2: u64,
    // ... more args
) -> SyscallResult {
    match num {
        SyscallNumber::Read => handlers::read(arg1, arg2, arg3),
        SyscallNumber::Write => handlers::write(arg1, arg2, arg3),
        // ... more syscalls
    }
}
```

### 7.3 Example: `read` Syscall

**File:** `kernel/src/syscall/handlers.rs`

```rust
pub fn read(fd: u64, buffer: u64, size: u64) -> SyscallResult {
    // 1. Validate file descriptor
    // 2. Get file handle
    // 3. Read data into buffer
    // 4. Return bytes read
}
```

**Flow:**
```
Program calls read() → syscall(READ, fd, buf, size) → Kernel handler → Read from file → Return bytes
```

---

## Key Rust Concepts Used

### 1. `no_std`

**What it means:**
- Rust normally uses a **standard library** (`std`) with features like:
  - File I/O
  - Networking
  - Threading
  - Allocators
- **`no_std`** means "don't use the standard library"
- Why? The standard library assumes an OS exists, but we're **building** the OS!

**What we provide instead:**
- Custom allocator (heap)
- Custom threading (scheduler)
- Custom I/O (UART, drivers)

### 2. `unsafe`

**What it means:**
- Rust normally prevents dangerous operations (like accessing raw memory)
- **`unsafe`** says "I know what I'm doing, allow dangerous operations"
- Used for:
  - Direct memory access (UART registers)
  - Assembly code (context switching)
  - Raw pointers

**Example:**
```rust
unsafe {
    let value = (0x0900_0000 as *mut u32).read_volatile();
    // Directly reading from memory address (UART register)
}
```

### 3. `Mutex` (Mutual Exclusion)

**What it means:**
- A **mutex** ensures only one thread accesses data at a time
- Prevents **race conditions** (bugs from concurrent access)

**Example:**
```rust
static UART: Mutex<Uart> = Mutex::new(Uart::new(UART_BASE));

// Thread 1
let uart = UART.lock();  // Acquire lock
uart.write_byte(b'A');  // Use UART
// Lock released when uart goes out of scope

// Thread 2
let uart = UART.lock();  // Waits if Thread 1 has lock
uart.write_byte(b'B');
```

**Why needed?**
- If two threads write to UART simultaneously, output would be garbled
- Mutex ensures only one thread writes at a time

### 4. `Result<T, E>`

**What it means:**
- Rust's way of handling errors
- `Result<OkType, ErrorType>`
- Either `Ok(value)` or `Err(error)`

**Example:**
```rust
fn mkdir(path: &str) -> Result<(), FsError> {
    if path_exists(path) {
        return Err(FsError::AlreadyExists);
    }
    create_directory(path)?;  // ? means "if error, return it"
    Ok(())  // Success
}

// Usage:
match mkdir("test") {
    Ok(()) => println!("Created!"),
    Err(e) => println!("Error: {}", e),
}
```

### 5. Lifetimes (`'a`)

**What it means:**
- Rust tracks how long data lives in memory
- Prevents **use-after-free** bugs (accessing freed memory)

**Example:**
```rust
fn get_string() -> &str {  // Error! What does this reference point to?
    "hello"  // This string is destroyed when function returns
}

// Fix: Return owned String
fn get_string() -> String {
    String::from("hello")
}
```

### 6. Ownership

**What it means:**
- Each value has **one owner**
- When owner goes out of scope, value is freed
- Prevents memory leaks and double-frees

**Example:**
```rust
let s1 = String::from("hello");
let s2 = s1;  // s1 is moved to s2, s1 is no longer valid
// println!("{}", s1);  // Error! s1 was moved

// To copy instead of move:
let s2 = s1.clone();  // Now both s1 and s2 are valid
```

---

## Complete Example: Typing "mkdir test" and Pressing Enter

Let's trace the **complete flow** from keyboard to directory creation:

### Step 1: You Type 'm'
```
Keyboard → Terminal → QEMU → UART Register → Shell's read_char() → 'm' added to buffer
```

### Step 2: You Type More Characters
```
'm' → 'k' → 'd' → 'i' → 'r' → ' ' → 't' → 'e' → 's' → 't' → '\n' (Enter)
```

### Step 3: Shell Reads Line
```
read_line() → Checks UART → Builds string "mkdir test\n" → Trims to "mkdir test"
```

### Step 4: Shell Parses Command
```
execute("mkdir test") → Split → cmd="mkdir", args=["test"]
```

### Step 5: Call mkdir Handler
```
commands::mkdir(["test"]) → fs::mkdir("test")
```

### Step 6: VFS Creates Directory
```
vfs::mkdir("test") → Parse path → Find parent → Create node → Add to parent
```

### Step 7: Return Success
```
mkdir() → Ok(()) → Shell continues → Prints new prompt
```

**Complete Flow Diagram:**
```
Keyboard Input
    ↓
Terminal (host)
    ↓
QEMU (virtualizes UART)
    ↓
UART Register (hardware)
    ↓
Shell read_char() (polls UART)
    ↓
Shell read_line() (builds string)
    ↓
Shell execute() (parses command)
    ↓
commands::mkdir() (command handler)
    ↓
fs::mkdir() (VFS layer)
    ↓
RamFS::create_directory() (filesystem driver)
    ↓
Directory Created!
    ↓
Return to Shell
    ↓
Print New Prompt
```

---

## Common Patterns in DebOS

### Pattern 1: Initialization Functions

Many modules have an `init()` function:
- `scheduler::init()`
- `memory::init_aarch64()`
- `drivers::init()`

**Why?** Sets up global state before use.

### Pattern 2: Static Global State

```rust
static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
static UART: Mutex<Uart> = Mutex::new(Uart::new(UART_BASE));
```

**Why?** Need global access, but Rust requires mutex for thread safety.

### Pattern 3: Architecture-Specific Code

```rust
#[cfg(target_arch = "aarch64")]
fn do_something() { /* ARM code */ }

#[cfg(target_arch = "x86_64")]
fn do_something() { /* x86 code */ }
```

**Why?** Different CPU architectures need different code.

### Pattern 4: Error Handling

```rust
fn operation() -> Result<Success, Error> {
    if something_bad() {
        return Err(Error::SomethingBad);
    }
    Ok(Success)
}
```

**Why?** Rust's safe error handling (no exceptions).

---

## Summary: The Big Picture

1. **Boot**: Assembly sets up CPU, Rust initializes everything
2. **Memory**: Page tables, allocators set up
3. **Scheduler**: Thread management, context switching
4. **Shell**: Interactive command interface
5. **Commands**: Parse and execute user commands
6. **Filesystem**: VFS + drivers for file operations
7. **Syscalls**: Interface for programs to request OS services

**Everything works together:**
- Timer interrupts → Scheduler → Context switch → Shell runs
- Keyboard input → UART → Shell reads → Command executes
- Command → VFS → Filesystem → Directory created

**Key Takeaway:** DebOS is a **microkernel** - the kernel does minimal work, most features are separate programs that communicate via IPC (Inter-Process Communication).

---

## Next Steps for Learning

1. **Read the code**: Start with `kernel/src/main.rs`, then `kernel/src/lib.rs`
2. **Trace a command**: Pick a command (like `ls`), trace it through the code
3. **Add a command**: Try adding a simple command like `echo`
4. **Understand scheduling**: Read `kernel/src/scheduler/mod.rs` carefully
5. **Study memory**: Read `kernel/src/memory/mod.rs` to understand allocators

**Recommended Reading Order:**
1. `kernel/src/main.rs` - Boot process
2. `kernel/src/lib.rs` - Initialization
3. `kernel/src/shell/mod.rs` - Shell main loop
4. `kernel/src/shell/commands.rs` - Command implementations
5. `kernel/src/scheduler/mod.rs` - Thread scheduling
6. `kernel/src/fs/vfs.rs` - Filesystem interface

---

## Glossary

- **BSS**: Block Started by Symbol - uninitialized data (set to zero)
- **Context Switch**: Saving one thread's state, loading another's
- **GIC**: Generic Interrupt Controller (ARM)
- **IPC**: Inter-Process Communication
- **Microkernel**: Kernel design with minimal functionality
- **Mutex**: Mutual exclusion lock (prevents concurrent access)
- **Page Table**: Maps virtual addresses to physical addresses
- **Preemption**: Forcing a thread to stop and switch to another
- **Scheduler**: Decides which thread runs when
- **Syscall**: System call - program requests OS service
- **UART**: Serial port for input/output
- **VFS**: Virtual Filesystem - unified interface for filesystems

---

**End of Beginner's Guide**

For more details, see:
- `README.md` - Quick start and overview
- `IMPLEMENTATION_PLAN.md` - Detailed development plan
- `docs/developer/` - Technical deep-dives



