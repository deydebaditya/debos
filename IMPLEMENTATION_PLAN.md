# DebOS Implementation Plan

> **Document Version:** 1.0.0  
> **Based on:** DebOS Technical Reference v2.1.0  
> **Created:** November 28, 2025

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [Phase 1: Kernel Parity](#phase-1-kernel-parity-months-1-3)
4. [Phase 2: Core Drivers](#phase-2-core-drivers-months-4-5)
5. [Phase 3: The AI Layer](#phase-3-the-ai-layer-months-6-8)
6. [Phase 4: Advanced Concurrency](#phase-4-advanced-concurrency-independent)
7. [Project Structure](#project-structure)
8. [Development Environment](#development-environment)
9. [Detailed TODO Lists](#detailed-todo-lists)

---

## Executive Summary

DebOS is a **POSIX-compatible microkernel** system written in Rust with AI integration capabilities. The system follows a microkernel design philosophy where the kernel (DeK - DebOS Nano-Kernel) provides only bare minimum mechanisms, while all OS functionality runs in userspace servers.

### Key Differentiators

- **Microkernel Architecture**: Superior security and stability vs monolithic kernels
- **Rust-based**: Memory safety guarantees via `no_std` Rust
- **AI-First**: Intent Engine and Generative UI as first-class citizens
- **Capability-based Security**: Fine-grained access control
- **Advanced Concurrency**: Green threading, work-stealing scheduler, unified CPU/GPU compute

---

## Architecture Overview

### System Layers

```
┌─────────────────────────────────────────────────────────────┐
│                    Ring 3 (Applications)                     │
│                 User apps → libdebos (std lib)               │
├─────────────────────────────────────────────────────────────┤
│                    Ring 3 (Core Servers)                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────┐  │
│  │VFS Server│ │NetServer │ │ DevMan   │ │ Window Server  │  │
│  │(FS mgmt) │ │(TCP/IP)  │ │(Hardware)│ │ (Compositor)   │  │
│  └──────────┘ └──────────┘ └──────────┘ └────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    Ring 0 (Kernel - DeK)                     │
│         Interrupts │ Scheduling │ IPC │ Memory Mgmt          │
└─────────────────────────────────────────────────────────────┘
```

### Core Components Summary

| Component | Location | Description |
|-----------|----------|-------------|
| DeK (DebOS Nano-Kernel) | Ring 0 | Core kernel: scheduling, IPC, memory |
| VFS Server | Ring 3 | Filesystem management (FAT32, ext4, DebFS) |
| Net Server | Ring 3 | TCP/IP networking stack |
| Device Manager | Ring 3 | Hardware enumeration, driver loading |
| Window Server | Ring 3 | Display compositor, GenUI stream |
| Intent Engine | Ring 3 | AI-powered input processing |
| GenShell | Ring 3 | Generative UI system |

---

## Phase 1: Kernel Parity (Months 1-3)

**Goal:** Boot, Memory Management, Preemptive Scheduler  
**Test Criteria:** Run 2 threads that print to VGA console simultaneously

### 1.1 Boot Infrastructure

#### 1.1.1 Bootloader Integration
- [ ] Set up x86_64 bootloader (UEFI or legacy BIOS support)
- [ ] Create multiboot2 header for GRUB compatibility
- [ ] Initialize early console output (VGA text mode / serial)
- [ ] Parse memory map from bootloader

#### 1.1.2 Early Initialization
- [ ] Set up GDT (Global Descriptor Table)
  - Null descriptor
  - Kernel code segment (64-bit)
  - Kernel data segment
  - User code segment (64-bit)
  - User data segment
  - TSS (Task State Segment)
- [ ] Set up IDT (Interrupt Descriptor Table)
  - CPU exceptions (0-31)
  - Hardware interrupts (32-47)
  - System call entry (0x80 or syscall instruction)
- [ ] Initialize PIC or APIC
- [ ] Enable interrupts

### 1.2 Memory Management

#### 1.2.1 Physical Memory Manager
- [ ] Implement Buddy Allocator for page frame management
  - Order-based allocation (4KB to 4MB blocks)
  - Free list per order
  - Coalescing on free
- [ ] Parse bootloader memory map
- [ ] Track available/reserved regions

#### 1.2.2 Virtual Memory Manager
- [ ] Implement 4-level page table management (PML4 → PDPT → PD → PT)
- [ ] Recursive page table mapping for self-reference
- [ ] CR3 register management
- [ ] Page fault handler

#### 1.2.3 Kernel Heap
- [ ] Implement Slab Allocator for internal kernel objects
  - TCBs (Thread Control Blocks)
  - Endpoints (IPC)
  - Capability nodes
- [ ] Global allocator trait implementation

### 1.3 Threading & Scheduling

#### 1.3.1 Thread Management
- [ ] Define `ARCH_CONTEXT` struct:
  ```rust
  pub struct ArchContext {
      pub rip: u64,      // Instruction pointer
      pub rsp: u64,      // Stack pointer
      pub rflags: u64,   // CPU flags
      // General Purpose Registers
      pub rax: u64, pub rbx: u64, pub rcx: u64, pub rdx: u64,
      pub rsi: u64, pub rdi: u64, pub rbp: u64,
      pub r8: u64, pub r9: u64, pub r10: u64, pub r11: u64,
      pub r12: u64, pub r13: u64, pub r14: u64, pub r15: u64,
      // Segment selectors, etc.
  }
  ```
- [ ] Implement Thread Control Block (TCB)
- [ ] Thread state machine (Ready, Running, Blocked, Terminated)

#### 1.3.2 Scheduler
- [ ] Preemptive Priority Round-Robin scheduler
- [ ] O(1) bitmap queue implementation
- [ ] Priority classes:
  - Real-Time: 0-31 (hard priority, no starvation guarantee)
  - Normal: 32-255 (dynamic priority with aging)
- [ ] Core affinity (soft-pinning for cache locality)
- [ ] Timer interrupt for preemption

#### 1.3.3 Context Switching
- [ ] Implement `context_switch(old: *mut Context, new: *const Context)` in assembly
- [ ] Save/restore FPU/SSE state
- [ ] Handle kernel/user mode transitions

### 1.4 System Call Interface (Foundation)

#### 1.4.1 Syscall Infrastructure
- [ ] Set up `syscall` instruction handler (STAR, LSTAR, SFMASK MSRs)
- [ ] Implement `syscall_dispatcher.rs`
- [ ] Capability validation framework

#### 1.4.2 Thread Syscalls
- [ ] `sys_thread_spawn(entry_point, stack_ptr, priority, capability_cptr) -> tid`
- [ ] `sys_thread_yield()`
- [ ] `sys_thread_exit(exit_code)`

#### 1.4.3 Memory Syscalls
- [ ] `sys_mem_map(frame_cap, page_dir_cap, virt_addr, flags) -> result`
  - Flags: READ | WRITE | EXECUTE | USER

### 1.5 Inter-Process Communication (IPC)

**Critical:** This is the "heart" of DebOS, replacing function calls in monolithic kernels.

#### 1.5.1 IPC Primitives
- [ ] `sys_ipc_call(endpoint_cap, msg_ptr, len, reply_buf_ptr, reply_len)`
  - Atomic send + block for reply
  - RPC-style calls to servers
- [ ] `sys_ipc_wait(endpoint_cap, buffer_ptr)`
  - Server-side listener

#### 1.5.2 Optimizations
- [ ] **Direct Switch Optimization**: If target thread is waiting, switch CPU context directly to it without full scheduler requeue (L4-style IPC)
- [ ] Zero-copy message passing for large payloads

### 1.6 Hardware Abstraction

- [ ] `sys_irq_ack(irq_number)` - Interrupt acknowledgment for userspace drivers
- [ ] Basic IRQ routing to userspace

---

## Phase 2: Core Drivers (Months 4-5)

**Goal:** PCI Enumeration, VirtIO-Block, VirtIO-Net  
**Test Criteria:** Mount an ext4 disk image and read a file. Ping `8.8.8.8`.

### Phase 2 Sub-phases

| Phase | Description | Status |
|-------|-------------|--------|
| 2A | In-kernel RamFS + shell commands | ✅ Complete |
| 2B | VirtIO subsystem + Block driver | ✅ Complete |
| 2C | FAT32 filesystem support | ✅ Complete |
| 2D | VFS Server migration (userspace) | ⏳ Pending |
| 2E | Networking stack | ⏳ Pending |
| 2F | **Ultra-Fast I/O (100x improvement)** | ⏳ Pending |

### 2.1 Device Manager (DevMan)

#### 2.1.1 Bus Enumeration
- [ ] PCIe ECAM (Enhanced Configuration Access Mechanism) scanner
- [ ] USB enumeration framework (future)
- [ ] Device tree construction

#### 2.1.2 Driver Loading
- [ ] Driver binary format (WASM/ELF)
- [ ] Driver discovery (match vendor/device IDs)
- [ ] Spawn driver process with isolated address space

#### 2.1.3 Capability Granting
- [ ] MMIO region capability generation
- [ ] IRQ line capability assignment
- [ ] DMA buffer management

### 2.1B VirtIO Subsystem (Kernel-mode, Phase 2B) ✅ COMPLETE

#### 2.1B.1 VirtIO Core
- [x] VirtQueue implementation (split virtqueues)
- [x] Descriptor table management
- [x] Available/Used ring handling
- [x] Memory barriers and cache coherency
- [x] Device status negotiation

#### 2.1B.2 VirtIO MMIO Transport
- [x] Device discovery via MMIO (for QEMU virt machine)
- [x] Register interface implementation (legacy v1 + modern v2)
- [x] Interrupt handling

#### 2.1B.3 VirtIO-Block Driver
- [x] Block device capability negotiation
- [x] Read/write request handling
- [x] Sector-based I/O interface
- [ ] Async I/O with completion callbacks (future enhancement)

### 2.2 Storage Stack (VFS Server)

#### 2.2.1 VFS Protocol
```
VFS_OPEN(path, flags)   -> handle
VFS_READ(handle, length) -> data
VFS_WRITE(handle, data)  -> result
VFS_CLOSE(handle)        -> result
VFS_STAT(path)           -> metadata
VFS_MKDIR(path)          -> result
VFS_UNLINK(path)         -> result
```

#### 2.2.2 Filesystem Drivers
- [ ] **FAT32**: Basic compatibility layer
- [ ] **ext4**: Full read/write support
- [ ] **DebFS**: Native filesystem (future)

#### 2.2.3 Storage Drivers
- [ ] **VirtIO-Block**: Virtualization support
  - Virtqueue initialization
  - Request/completion handling
- [ ] **NVMe Driver** (Stretch goal)
  - Submission/Completion Queues
  - Shared DMA memory

### 2.3 Networking Stack (NetServer)

#### 2.3.1 Network Protocol
```
NET_SOCKET(domain, type)   -> handle
NET_BIND(handle, ip, port) -> result
NET_LISTEN(handle)         -> result
NET_ACCEPT(handle)         -> new_handle
NET_CONNECT(handle, ip, port) -> result
NET_SEND(handle, data)     -> bytes_sent
NET_RECV(handle, length)   -> data
NET_CLOSE(handle)          -> result
```

#### 2.3.2 Protocol Stack
- [ ] Ethernet frame handling
- [ ] ARP (Address Resolution Protocol)
- [ ] IPv4 (IPv6 future)
- [ ] ICMP (ping support)
- [ ] UDP
- [ ] TCP (full state machine)

#### 2.3.3 Zero-Copy Architecture
- [ ] Ring buffer implementation
- [ ] Shared memory regions between driver and NetServer

#### 2.3.4 Network Drivers
- [ ] **VirtIO-Net**: Primary virtual NIC
  - Virtqueue setup (RX/TX)
  - Interrupt handler loop (`sys_irq_wait`)
  - Packet forwarding to NetServer via IPC
- [ ] **Intel e1000**: Broad compatibility
- [ ] **RTL8139**: Legacy support

### 2.4 Driver Development Template

For each driver (e.g., `drivers/virtio_net`):

1. [ ] Map PCI BAR region
2. [ ] Initialize device-specific structures (Virtqueues, etc.)
3. [ ] Implement interrupt handler loop
4. [ ] Data transfer to/from core servers via IPC
5. [ ] Error handling and recovery

### 2.5 Ultra-Fast I/O (Phase 2F)

**Goal:** 100x faster I/O than existing operating systems  
**Documentation:** [docs/developer/ULTRA_FAST_IO.md](docs/developer/ULTRA_FAST_IO.md)

#### 2.5.1 IoRing - Async I/O Engine

**Core Concept:** io_uring-style lock-free submission/completion queues

```
┌──────────────────────────────────────────────────────────────┐
│                    Traditional I/O Path                       │
│  Syscall (1-2µs) → VFS (0.5µs) → FS (0.5µs) → Block (0.5µs) │
│  → Driver (0.5µs) → Interrupt (2-5µs)                        │
│  Total: 5-10µs per I/O                                        │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│                    DebOS IoRing Path                          │
│  Memory write to SQ (20ns) → Poll CQ (10ns)                  │
│  Total: 30-50ns per I/O                                       │
└──────────────────────────────────────────────────────────────┘
```

- [ ] **IORING-001**: Submission Queue (SQ) - lock-free ring buffer
- [ ] **IORING-002**: Completion Queue (CQ) - lock-free ring buffer
- [ ] **IORING-003**: SQ Entry (SQE) - 64-byte command structure
- [ ] **IORING-004**: CQ Entry (CQE) - 16-byte completion structure
- [ ] **IORING-005**: sys_io_ring_setup syscall
- [ ] **IORING-006**: sys_io_ring_enter syscall
- [ ] **IORING-007**: sys_io_ring_register syscall
- [ ] **IORING-008**: READ/WRITE operation support
- [ ] **IORING-009**: FSYNC operation support
- [ ] **IORING-010**: Linked operations (chains)

#### 2.5.2 Zero-Copy Buffer Management

**Core Concept:** Pre-registered buffers, no per-I/O allocations

- [ ] **ZCOPY-001**: Buffer pool allocator (lock-free)
- [ ] **ZCOPY-002**: DMA mapping cache
- [ ] **ZCOPY-003**: IOMMU integration for DMA safety
- [ ] **ZCOPY-004**: Fixed file descriptor table
- [ ] **ZCOPY-005**: Registered buffer syscalls
- [ ] **ZCOPY-006**: Splice/sendfile zero-copy

#### 2.5.3 Polled I/O Engine

**Core Concept:** Eliminate interrupt overhead with polling

```
Interrupt-driven: Device completion → IRQ (1µs) → Handler (0.5µs) 
                  → Context switch (1-2µs) = 3-5µs total

Polled mode:      Device completion → Poll CQ (10-20ns) = 10-20ns total
```

- [ ] **POLL-001**: Poll-mode NVMe driver
- [ ] **POLL-002**: Per-CPU poll threads
- [ ] **POLL-003**: Adaptive polling (poll → interrupt on idle)
- [ ] **POLL-004**: CPU affinity for poll threads
- [ ] **POLL-005**: Power management integration

#### 2.5.4 Batched Submission

**Core Concept:** Amortize syscall overhead across many operations

- [ ] **BATCH-001**: Multi-submit syscall (32+ ops per call)
- [ ] **BATCH-002**: Vectored I/O (readv/writev)
- [ ] **BATCH-003**: Coalesced completions
- [ ] **BATCH-004**: Timeout handling for batches

#### 2.5.5 Lock-Free Filesystem

**Core Concept:** RCU-based metadata, per-CPU allocation

- [ ] **LFFS-001**: RCU (Read-Copy-Update) infrastructure
- [ ] **LFFS-002**: Lock-free inode cache
- [ ] **LFFS-003**: Lock-free dentry cache
- [ ] **LFFS-004**: Per-CPU block allocation
- [ ] **LFFS-005**: Lock-free free space bitmap
- [ ] **LFFS-006**: Lazy writeback with batching

#### 2.5.6 Security Enforcement Layer

**Core Concept:** All I/O validated by kernel, no exceptions

- [ ] **SEC-001**: Per-ring capability enforcement
- [ ] **SEC-002**: Per-operation permission checking (cached)
- [ ] **SEC-003**: Buffer address validation (pre-registration)
- [ ] **SEC-004**: IOMMU-enforced DMA boundaries
- [ ] **SEC-005**: Rate limiting per process
- [ ] **SEC-006**: Audit logging for I/O operations
- [ ] **SEC-007**: Ring isolation between processes

#### 2.5.7 Performance Targets (WITH Full Security)

| Metric | Linux io_uring | DebOS Target | Improvement |
|--------|----------------|--------------|-------------|
| Submit latency | 100-200ns | 20-50ns | 4-10x |
| Completion latency | 50-100ns | 10-20ns | 5-10x |
| End-to-end 4KB read | 5-10µs | 100-300ns | 30-100x |
| Random 4KB IOPS | 1-2M | 5-10M | 5-10x |
| Batched submit (32) | 3-5µs | 100-200ns | 25-50x |

#### 2.5.8 Trade-offs (Security NOT Compromised)

| Trade-off | Impact | Mitigation |
|-----------|--------|------------|
| ~~Security~~ | **NONE** | Full kernel isolation maintained |
| Memory overhead | +50-100MB | Only for fast-path apps |
| Power consumption | +10-30% when polling | Adaptive polling |
| Complexity | +5000 LOC | Extensive testing, documentation |
| Legacy compat | Must use new APIs | Transparent wrapper available |

> ✅ **SECURITY GUARANTEE:** DebOS Ultra-Fast I/O maintains strict kernel isolation.
> All operations are validated by the kernel. No userspace access to devices.

---

## Phase 3: The AI Layer (Months 6-8)

**Goal:** Port ONNX Runtime, Intent Engine, GenShell  
**Test Criteria:** "Draw" a calculator UI based on a text prompt

### 3.1 Intent Engine (Service)

**Purpose:** Replaces standard Input subsystem

#### 3.1.1 Architecture
```
┌─────────────┐     ┌──────────────────┐     ┌────────────────┐
│ HID Events  │ ──▶ │  Intent Engine   │ ──▶ │ Active App     │
│ (kbd/mouse) │     │ (Transformer ML) │     │ (IntentEvents) │
└─────────────┘     └──────────────────┘     └────────────────┘
```

#### 3.1.2 Implementation
- [ ] High-priority thread loop
- [ ] Load `model.onnx` from initramfs
- [ ] Read from `/dev/input/events`
- [ ] Feed tensor data to model
- [ ] Emit IntentEvents when probability > 0.8:
  - `INTENT_SCROLL_DOWN`
  - `INTENT_OPEN_APP`
  - `INTENT_SUMMARIZE`
  - etc.
- [ ] IPC send to Active Application

#### 3.1.3 Performance Requirements
- [ ] < 16ms latency (60Hz loop)
- [ ] Quantized model for efficiency

### 3.2 ONNX Runtime Integration

- [ ] Port ONNX Runtime Lite to DebOS
- [ ] `no_std` compatibility layer
- [ ] CPU inference backend (AVX2/AVX-512)
- [ ] Memory-efficient tensor allocation

### 3.3 Generative UI (GenShell)

**Purpose:** Replaces traditional Desktop Environment (GNOME/KDE)

#### 3.3.1 Philosophy
- **No static widgets**: No predefined buttons/windows stored on disk
- **Dynamic rendering**: Apps send JSON state, GenShell generates UI

#### 3.3.2 Implementation
- [ ] JSON state protocol definition
- [ ] Layout engine / diffusion-style model
- [ ] Real-time UI frame generation
- [ ] Compositor integration

#### 3.3.3 Application Protocol
```json
{
  "app_id": "calculator",
  "state": {
    "display": "123.45",
    "operation": "add",
    "buttons": ["0-9", "+", "-", "*", "/", "=", "C"]
  },
  "intent": "render_calculator_interface"
}
```

### 3.4 Window Server (Compositor)

- [ ] Display management
- [ ] GenUI stream rendering
- [ ] Input event routing
- [ ] Multi-window composition (if applicable)

---

## Phase 4: Advanced Concurrency (Independent)

**Goal:** Make DebOS the most capable multi-processing and multi-threaded OS  
**Status:** Independent phase, can run in parallel with other phases  
**Test Criteria:** Spawn 10 million green threads, achieve sub-100ns context switching

> 📖 **Full Documentation:** See [docs/developer/CONCURRENCY_IMPLEMENTATION.md](docs/developer/CONCURRENCY_IMPLEMENTATION.md)

### 4.1 Green Threading Core

**M:N threading model with millions of lightweight threads**

- [ ] `GreenThread` structure with minimal context (~64 bytes)
- [ ] `GreenContext` save/restore (x86_64 and AArch64)
- [ ] `GrowableStack` with guard pages (2KB initial, up to 1MB)
- [ ] Ultra-fast context switching (~50-100ns)
- [ ] Basic spawn/yield/exit operations

### 4.2 Work-Stealing Scheduler

**Lock-free, NUMA-aware work distribution**

- [ ] Chase-Lev lock-free work-stealing deque
- [ ] Per-core executors with local run queues
- [ ] Work stealing between cores
- [ ] NUMA topology detection
- [ ] NUMA-aware stealing (prefer local node)
- [ ] Global injection queue for external events

### 4.3 Async I/O Integration

**io_uring-style completion-based I/O**

- [ ] IoRing submission/completion queues
- [ ] Zero-copy I/O operations
- [ ] Green thread I/O blocking/wake-up
- [ ] Batched syscall submission
- [ ] Async file and socket operations

### 4.4 GPU Compute Integration (Opt-in, Disabled by Default)

**Unified CPU/GPU compute model - must be enabled at boot**

> ⚠️ GPU compute is **disabled by default**. Enable via: `gpu_compute=enabled`

- [ ] Boot parameter parsing (`gpu_compute=enabled|disabled`)
- [ ] GPU device enumeration (only when enabled)
- [ ] Unified memory addressing (CPU/GPU shared)
- [ ] GPU task submission and scheduling
- [ ] Automatic CPU/GPU work partitioning
- [ ] Metal backend (Apple Silicon/macOS)
- [ ] Vulkan compute backend (Linux/Windows)
- [ ] Graceful CPU-only fallback

### 4.5 User-Space API (libdebos)

```rust
// Green thread spawning
let handle = spawn_green(|| compute_something());

// Parallel iterators with work-stealing
data.par_iter().map(|x| x * 2).sum();

// Structured concurrency
scope(|s| {
    s.spawn(|| task_a());
    s.spawn(|| task_b());
}); // Both complete before scope exits

// Async I/O (yields green thread, not OS thread)
let bytes = file.read(&mut buf).await?;

// GPU compute (auto CPU/GPU partitioning)
parallel_compute(&mut data, |x| *x = (*x).sqrt());
```

### Performance Targets

| Metric | Target | Comparison |
|--------|--------|------------|
| Green thread spawn | < 100ns | 200x faster than pthread |
| Context switch | < 100ns | 15x faster than Linux |
| Memory per thread | 2KB | 1000x less than pthread |
| Max concurrent | 10M+ | 1000x more than typical |
| Work steal latency | < 500ns | Lock-free |

---

## Phase 5: User Management & Security (Months 9-11)

**Goal:** Complete multi-user OS with enterprise-grade security  
**Status:** Pending  
**Test Criteria:** Multiple users with isolated processes, file permissions enforced

> 📖 **Full Documentation:** See [docs/developer/USER_SECURITY_SYSTEM.md](docs/developer/USER_SECURITY_SYSTEM.md)

### 5.1 User Identity System

**Core Concept:** Users and groups with POSIX-compatible ID system

- [ ] UserId type (0=root, 1-999=system, 1000+=regular)
- [ ] GroupId type with similar ranges
- [ ] User account structure (username, home, shell, etc.)
- [ ] /etc/passwd and /etc/shadow equivalents
- [ ] User database manager

### 5.2 Group System

**Core Concept:** Group membership for shared access

- [ ] Group structure with members and admins
- [ ] Primary and supplementary groups
- [ ] /etc/group and /etc/gshadow equivalents
- [ ] Default system groups (root, wheel, users, etc.)
- [ ] Maximum 32 supplementary groups per process

### 5.3 Authentication System

**Core Concept:** Secure password-based authentication with Argon2id

- [ ] Password hashing with Argon2id (64MB, 3 iterations)
- [ ] Salt generation (128-bit random)
- [ ] Constant-time password comparison
- [ ] Login program (identification → authentication → session)
- [ ] Failed login handling (exponential backoff, lockout)
- [ ] Session management
- [ ] PAM-like module interface

### 5.4 Process Credentials

**Core Concept:** Each process has user/group credentials

```
Real UID/GID     - Who started the process
Effective UID/GID - Used for permission checks
Saved UID/GID    - For setuid programs
Filesystem UID/GID - For file access
```

- [ ] ProcessCredentials struct
- [ ] Credential inheritance on fork()
- [ ] Credential transition on exec()
- [ ] setuid/setgid/setgroups syscalls
- [ ] getuid/getgid/getgroups syscalls

### 5.5 File Ownership & Permissions

**Core Concept:** POSIX file permissions with owner/group/other

```
-rwxr-xr-x  1 root wheel  1024 Jan 1 12:00 /usr/bin/ls
│││ │││ │││
│││ │││ └── Other: r-x (read, execute)
│││ └───── Group: r-x (read, execute)  
└──────── Owner: rwx (read, write, execute)
```

- [ ] File mode (rwxrwxrwx) in inode
- [ ] Owner UID and GID in inode
- [ ] Setuid/setgid/sticky bits
- [ ] Permission checking algorithm
- [ ] chown/chmod/chgrp syscalls
- [ ] umask support

### 5.6 Capability System

**Core Concept:** Fine-grained privileges replacing all-or-nothing root

| Capability | Purpose |
|------------|---------|
| CAP_DAC_OVERRIDE | Bypass file permissions |
| CAP_SETUID | Change process UID |
| CAP_NET_BIND_SERVICE | Bind to ports < 1024 |
| CAP_SYS_ADMIN | Various administrative operations |

- [ ] CapabilitySet bitmap type
- [ ] Per-process capability sets (permitted, effective, inheritable, bounding)
- [ ] Capability inheritance on fork/exec
- [ ] Capability-aware syscalls
- [ ] File capabilities (optional)

### 5.7 Superuser (Root) Model

**Core Concept:** Root (UID 0) has all capabilities by default

- [ ] Root bypass for permission checks
- [ ] Root login restrictions (securetty)
- [ ] su command (switch user)
- [ ] sudo command (execute as another user)
- [ ] Capability dropping for least privilege

### 5.8 Security Policies

**Core Concept:** Defense in depth with multiple security layers

- [ ] Resource limits (max processes, open files, memory)
- [ ] Mandatory Access Control labels (optional)
- [ ] Audit logging (login, permission denied, privilege use)
- [ ] Login restrictions (time, terminal, network)
- [ ] Account expiration

### 5.9 User Management Commands

| Command | Description |
|---------|-------------|
| `useradd` | Create new user |
| `userdel` | Delete user |
| `usermod` | Modify user |
| `passwd` | Change password |
| `groupadd` | Create group |
| `groupdel` | Delete group |
| `groupmod` | Modify group |
| `su` | Switch user |
| `sudo` | Execute as another user |
| `id` | Show current user/groups |
| `whoami` | Show current username |
| `chown` | Change file owner |
| `chmod` | Change file permissions |

---

## Project Structure

```
debos/
├── Cargo.toml                  # Workspace manifest
├── Dockerfile                  # Build environment
├── README.md
├── IMPLEMENTATION_PLAN.md      # This document
│
├── kernel/                     # DeK - DebOS Nano-Kernel
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # Kernel entry point
│       ├── arch/
│       │   ├── x86_64/
│       │   │   ├── mod.rs
│       │   │   ├── gdt.rs      # Global Descriptor Table
│       │   │   ├── idt.rs      # Interrupt Descriptor Table
│       │   │   ├── paging.rs   # Page table management
│       │   │   ├── context.rs  # ARCH_CONTEXT + context_switch
│       │   │   └── boot.asm    # Assembly bootstrap
│       │   └── aarch64/
│       │       ├── mod.rs
│       │       ├── boot.rs     # Boot sequence
│       │       ├── uart.rs     # PL011 UART driver
│       │       ├── exceptions.rs
│       │       ├── gic.rs      # GICv2 interrupt controller
│       │       ├── mmu.rs      # Memory management unit
│       │       └── context.rs  # Context switching
│       ├── memory/
│       │   ├── mod.rs
│       │   ├── buddy.rs        # Buddy allocator
│       │   ├── slab.rs         # Slab allocator
│       │   └── heap.rs         # Kernel heap
│       ├── scheduler/
│       │   ├── mod.rs
│       │   ├── thread.rs       # TCB definition
│       │   └── priority.rs     # O(1) bitmap queue
│       ├── ipc/
│       │   ├── mod.rs
│       │   ├── endpoint.rs     # IPC endpoints
│       │   └── message.rs      # Message structures
│       ├── syscall/
│       │   ├── mod.rs
│       │   ├── dispatcher.rs   # Syscall dispatcher
│       │   └── handlers.rs     # Individual syscall implementations
│       ├── shell/
│       │   ├── mod.rs          # Interactive kernel shell
│       │   ├── commands.rs     # Built-in commands
│       │   └── input.rs        # Input handling
│       └── capability/
│           └── mod.rs          # Capability system
│
├── libdebos/                   # Standard library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── syscall.rs          # Syscall wrappers
│       ├── thread.rs           # Thread API
│       ├── ipc.rs              # IPC abstractions
│       ├── fs.rs               # Filesystem API
│       └── net.rs              # Networking API
│
├── servers/                    # Userspace servers
│   ├── vfs/                    # Virtual Filesystem Server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── fat32.rs
│   │       └── ext4.rs
│   ├── netserver/              # Networking Server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── tcp.rs
│   │       ├── udp.rs
│   │       └── ip.rs
│   ├── devman/                 # Device Manager
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       └── pci.rs
│   └── intent_engine/          # AI Intent Engine
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           └── model.rs
│
├── drivers/                    # Userspace drivers
│   ├── virtio_block/
│   ├── virtio_net/
│   ├── nvme/
│   └── e1000/
│
├── genshell/                   # Generative UI
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
│
└── tools/                      # Build & utility tools
    ├── mkimage/                # Disk image creator
    └── initramfs/              # Initial ramdisk builder
```

---

## Development Environment

### Docker Build Environment

```dockerfile
FROM rust:latest
RUN apt-get update && apt-get install -y \
    qemu-system-x86 qemu-system-arm nasm mtools xorriso \
    clang lld llvm-dev \
    libclang-dev \
    # For generating FS images
    e2fsprogs dosfstools
WORKDIR /debos
COPY . .
CMD ["cargo", "build", "--release"]
```

### Required Tools

| Tool | Purpose |
|------|---------|
| `rust` (nightly) | Primary development language |
| `nasm` | x86_64 assembly |
| `qemu-system-x86` | x86_64 emulation and testing |
| `qemu-system-arm` | AArch64 emulation and testing |
| `xorriso` | ISO image creation |
| `mtools` | FAT filesystem manipulation |
| `e2fsprogs` | ext4 filesystem tools |
| `clang/lld` | Alternative linker, LLVM tools |

### Build Commands

```bash
# Build kernel for default architecture
make build

# Build for specific architectures
make build-x86        # x86_64
make build-arm        # AArch64

# Run in QEMU
make run-x86          # x86_64
make run-arm          # AArch64 (recommended for Apple Silicon)

# Run with VirtIO devices
qemu-system-x86_64 \
    -cdrom debos.iso \
    -m 512M \
    -device virtio-blk-pci,drive=hd0 \
    -drive file=disk.img,format=raw,id=hd0 \
    -device virtio-net-pci,netdev=net0 \
    -netdev user,id=net0
```

---

## Detailed TODO Lists

### Priority 1: Critical Path (Must Have) ✅ COMPLETE

- [x] **KERN-001**: Set up Rust `no_std` kernel crate with custom target
- [x] **KERN-002**: Implement GDT and TSS for x86_64
- [x] **KERN-003**: Implement IDT with exception handlers
- [x] **KERN-004**: Implement 4-level paging (CR3 management)
- [x] **KERN-005**: Implement Buddy Allocator
- [x] **KERN-006**: Implement basic heap allocator
- [x] **KERN-007**: Define `ArchContext` struct
- [x] **KERN-008**: Implement `context_switch` in assembly
- [x] **KERN-009**: Implement O(1) priority scheduler
- [x] **KERN-010**: Implement `syscall` instruction handler (x86_64 LSTAR/MSRs, AArch64 SVC)
- [x] **KERN-011**: Implement `sys_thread_spawn`
- [x] **KERN-012**: Implement `sys_thread_yield`
- [x] **KERN-013**: Implement `sys_thread_exit`
- [x] **KERN-014**: Implement `sys_mem_map` (with proper page table manipulation)
- [x] **KERN-015**: Implement `sys_ipc_call` with direct switch optimization (L4-style)
- [x] **KERN-016**: Implement `sys_ipc_wait`
- [x] **KERN-017**: Implement `sys_irq_ack` (PIC for x86_64, GIC for AArch64)

### Priority 2: Core Servers (Required for Functionality)

#### Phase 2A: In-Kernel Filesystem ✅ COMPLETE
- [x] **FS-001**: RamFS inode structure and operations
- [x] **FS-002**: VFS layer abstraction
- [x] **FS-003**: Path resolution utilities
- [x] **FS-004**: Shell commands (ls, cd, mkdir, rm, cat, etc.)

#### Phase 2B: VirtIO & Block Devices ✅ COMPLETE
- [x] **VIO-001**: VirtQueue core implementation
- [x] **VIO-002**: VirtIO MMIO transport (legacy v1 + modern v2)
- [x] **VIO-003**: VirtIO-Block driver
- [x] **VIO-004**: Block device abstraction layer

#### Phase 2C: FAT32 Filesystem ✅ COMPLETE
- [x] **FAT-001**: FAT32 boot sector parsing (BPB)
- [x] **FAT-002**: FAT table reading (cluster chains)
- [x] **FAT-003**: Directory entry parsing (8.3 filenames)
- [x] **FAT-004**: File read operations
- [x] **FAT-005**: File write operations (create, update, delete)
- [x] **FAT-006**: Cluster allocation and FAT entry management

#### Phase 2C+: Shell Utilities ✅ COMPLETE
- [x] **UTIL-001**: head command (first N lines)
- [x] **UTIL-002**: tail command (last N lines)
- [x] **UTIL-003**: grep command (pattern matching)
- [x] **UTIL-004**: vim-like text editor (edit command)

#### Phase 2D: Device Manager, Input & Networking ✅ COMPLETE

**Documentation:** [docs/developer/DEVICE_SUBSYSTEM.md](docs/developer/DEVICE_SUBSYSTEM.md)

##### Device Manager ✅
- [x] **DEV-001**: Device struct with id, class, bus, resources
- [x] **DEV-002**: Device tree with parent/child relationships
- [x] **DEV-003**: DeviceClass enum (Block, Keyboard, Mouse, Network, etc.)
- [x] **DEV-004**: BusType enum (Root, PCI, USB, Platform, VirtIO)
- [x] **DEV-005**: DeviceResources (MMIO, IRQ, DMA, I/O ports)
- [x] **DEV-006**: Driver registration and binding

##### Input Subsystem ✅
- [x] **INPUT-001**: InputEvent model (evdev-compatible)
- [x] **INPUT-002**: KeyCode module (USB HID compatible)
- [x] **INPUT-003**: Keyboard driver with modifier tracking
- [x] **INPUT-004**: Mouse driver with button/motion tracking
- [x] **INPUT-005**: PS/2 scancode translation
- [x] **INPUT-006**: Global input event queue

##### Networking Stack ✅
- [x] **NET-001**: MacAddress and Ipv4Address types
- [x] **NET-002**: NetworkInterface abstraction
- [x] **NET-003**: Ethernet frame handling
- [x] **NET-004**: ARP protocol with cache
- [x] **NET-005**: IPv4 protocol
- [x] **NET-006**: ICMP protocol (ping)
- [x] **NET-007**: UDP protocol
- [x] **NET-008**: TCP protocol (full state machine)
- [x] **NET-009**: Socket API (socket, bind, listen, connect, send, recv)

#### Phase 2D-TODO: Remaining Items
- [ ] **SRV-001**: VFS Server migration to userspace
- [x] **SRV-002**: ext4 filesystem driver
- [x] **USB-001**: xHCI controller driver
- [x] **USB-002**: USB device enumeration
- [x] **USB-003**: USB HID driver (keyboard/mouse)
- [x] **USB-004**: USB Mass Storage driver
- [x] **DISP-001**: Framebuffer abstraction (VirtIO-GPU)
- [x] **DISP-002**: VirtIO-GPU driver
- [x] **DISP-003**: Text console over framebuffer

#### Phase 2D-TODO: Infrastructure (Complete)
- [x] **INFRA-001**: PCI/PCIe enumeration driver
- [x] **INFRA-002**: USB subsystem with xHCI
- [x] **INFRA-003**: VirtIO-Net driver connected to NetworkInterface
- [x] **INFRA-004**: VirtIO-Input driver
- [x] **INFRA-005**: VirtIO-GPU driver
- [x] **NET-010**: Shell commands (ifconfig, ping, arp, netstat)
- [x] **NET-011**: Shell commands (devices, lspci, lsusb)

#### Phase 2F: Ultra-Fast I/O (100x Improvement) ⏳ PENDING

**Documentation:** [docs/developer/ULTRA_FAST_IO.md](docs/developer/ULTRA_FAST_IO.md)

##### 2F-1: IoRing Foundation
- [ ] **IORING-001**: SQ ring buffer (lock-free, cache-aligned)
- [ ] **IORING-002**: CQ ring buffer (lock-free, cache-aligned)
- [ ] **IORING-003**: SQE structure (64-byte, packed)
- [ ] **IORING-004**: CQE structure (16-byte, packed)
- [ ] **IORING-005**: sys_io_ring_setup (create ring, map memory)
- [ ] **IORING-006**: sys_io_ring_enter (submit, wait)
- [ ] **IORING-007**: sys_io_ring_register (buffers, files)
- [ ] **IORING-008**: READ operation
- [ ] **IORING-009**: WRITE operation
- [ ] **IORING-010**: FSYNC operation
- [ ] **IORING-011**: Linked operations (dependency chains)
- [ ] **IORING-012**: Timeout handling

##### 2F-2: Zero-Copy Infrastructure
- [ ] **ZCOPY-001**: Lock-free buffer pool allocator
- [ ] **ZCOPY-002**: Pre-registered buffer table
- [ ] **ZCOPY-003**: DMA address cache
- [ ] **ZCOPY-004**: IOMMU mapping for buffer pool
- [ ] **ZCOPY-005**: Fixed file descriptor table
- [ ] **ZCOPY-006**: Buffer select mechanism
- [ ] **ZCOPY-007**: Splice/sendfile support

##### 2F-3: Polled I/O Engine
- [ ] **POLL-001**: Poll-mode VirtIO-Block driver
- [ ] **POLL-002**: Poll-mode NVMe driver
- [ ] **POLL-003**: Per-CPU poll contexts
- [ ] **POLL-004**: Adaptive polling (busy → interrupt)
- [ ] **POLL-005**: CPU affinity for poll threads
- [ ] **POLL-006**: Polling power management

##### 2F-4: Batched Submission
- [ ] **BATCH-001**: Multi-submit (32+ SQEs per syscall)
- [ ] **BATCH-002**: Submit-and-wait (combined operation)
- [ ] **BATCH-003**: Drain semantics
- [ ] **BATCH-004**: Completion coalescing
- [ ] **BATCH-005**: Vectored I/O (readv/writev)

##### 2F-5: Lock-Free Filesystem
- [ ] **LFFS-001**: RCU core infrastructure
- [ ] **LFFS-002**: Epoch-based memory reclamation
- [ ] **LFFS-003**: Lock-free inode cache (hash table)
- [ ] **LFFS-004**: Lock-free dentry cache
- [ ] **LFFS-005**: Per-CPU block allocator
- [ ] **LFFS-006**: Lock-free free space bitmap
- [ ] **LFFS-007**: Lazy writeback queue

##### 2F-6: Security Enforcement (MANDATORY)
- [ ] **SEC-001**: Per-ring capability token validation
- [ ] **SEC-002**: Per-operation permission checking (with caching)
- [ ] **SEC-003**: Buffer pre-registration with address validation
- [ ] **SEC-004**: IOMMU integration for DMA protection
- [ ] **SEC-005**: Per-process I/O rate limiting
- [ ] **SEC-006**: Audit logging for all I/O operations
- [ ] **SEC-007**: Ring isolation (process cannot access other rings)
- [ ] **SEC-008**: Secure ring teardown on process exit

##### 2F-7: Benchmarking & Optimization
- [ ] **BENCH-001**: fio-compatible benchmark tool
- [ ] **BENCH-002**: Latency histogram collection
- [ ] **BENCH-003**: IOPS measurement
- [ ] **BENCH-004**: CPU profiling integration
- [ ] **BENCH-005**: Regression test suite
- [ ] **BENCH-006**: Comparison with Linux io_uring

### Priority 3: Drivers (Hardware Support)

- [x] **DRV-001**: VirtIO-Block driver
- [ ] **DRV-002**: VirtIO-Net driver
- [ ] **DRV-003**: NVMe driver (stretch)
- [ ] **DRV-004**: e1000 driver (stretch)

### Priority 4A: AI Layer (Differentiator)

- [ ] **AI-001**: Port ONNX Runtime Lite
- [ ] **AI-002**: Intent Engine service
- [ ] **AI-003**: HID event processing
- [ ] **AI-004**: Intent classification model
- [ ] **AI-005**: GenShell JSON protocol
- [ ] **AI-006**: GenShell renderer
- [ ] **AI-007**: Window compositor

### Priority 4B: Advanced Concurrency (Independent)

- [ ] **CONC-001**: GreenThread structure and context
- [ ] **CONC-002**: Ultra-fast context switching (x86_64)
- [ ] **CONC-003**: Ultra-fast context switching (AArch64)
- [ ] **CONC-004**: Growable stacks with guard pages
- [ ] **CONC-005**: Lock-free work-stealing deque
- [ ] **CONC-006**: Per-core executors
- [ ] **CONC-007**: Work-stealing scheduler
- [ ] **CONC-008**: NUMA topology detection
- [ ] **CONC-009**: NUMA-aware work stealing
- [ ] **CONC-010**: IoRing submission queue
- [ ] **CONC-011**: IoRing completion queue
- [ ] **CONC-012**: Green thread I/O integration
- [ ] **CONC-013**: GPU device enumeration
- [ ] **CONC-014**: Unified CPU/GPU memory
- [ ] **CONC-015**: GPU task scheduler
- [ ] **CONC-016**: libdebos green thread API
- [ ] **CONC-017**: libdebos parallel iterators
- [ ] **CONC-018**: libdebos structured concurrency

### Priority 4C: User Management & Security (Phase 5) ✅ COMPLETE

**Documentation:** [docs/developer/USER_SECURITY_SYSTEM.md](docs/developer/USER_SECURITY_SYSTEM.md)

##### 5A: Core Identity System ✅
- [x] **USER-001**: UserId type with ranges (root/system/regular)
- [x] **USER-002**: GroupId type with default groups
- [x] **USER-003**: User struct (uid, gid, username, home, shell)
- [x] **USER-004**: Group struct (gid, name, members, admins)
- [x] **USER-005**: In-memory user database (replaces /etc/passwd)
- [x] **USER-006**: In-memory group database (replaces /etc/group)
- [x] **USER-007**: User database manager (lookup, create, delete)

##### 5B: Process Credentials ✅
- [x] **CRED-001**: ProcessCredentials struct (uid, euid, suid, fsuid + groups)
- [x] **CRED-002**: Credential storage in TCB (Thread Control Block)
- [x] **CRED-003**: Credential inheritance on fork()
- [x] **CRED-004**: Credential transition on exec() (setuid handling)
- [x] **CRED-005**: set_euid / set_reuid / set_resuid
- [x] **CRED-006**: set_egid / set_groups
- [x] **CRED-007**: Scheduler integration (current_credentials, set_credentials)
- [x] **CRED-008**: Kernel credentials (root with all capabilities)
- [x] **CRED-009**: Default credentials (debos user)

##### 5C: Authentication System ✅
- [x] **AUTH-001**: Password hashing (simplified, Argon2id TODO)
- [x] **AUTH-002**: Salt generation (pseudo-random)
- [x] **AUTH-003**: Constant-time password comparison
- [x] **AUTH-004**: In-memory password database (root-only)
- [x] **AUTH-005**: PasswordEntry struct (hash, salt, expiry, lockout)
- [x] **AUTH-006**: Login command (authenticate and create session)
- [x] **AUTH-007**: Session management (session ID, login tracking)
- [x] **AUTH-008**: Failed login handling (exponential backoff)
- [x] **AUTH-009**: Account lockout after max failures
- [x] **AUTH-010**: Passwordless account support (debos user)

##### 5D: File Permissions ✅
- [x] **PERM-001**: Permission bits in Stat struct
- [x] **PERM-002**: Owner UID/GID in inode
- [x] **PERM-003**: AccessMode enum (Read, Write, Execute)
- [x] **PERM-004**: Permission checking algorithm (owner/group/other)
- [x] **PERM-005**: Root bypass with capability check
- [x] **PERM-006**: Default permissions (0644 files, 0755 dirs)
- [x] **PERM-007**: File ownership on creation (from credentials)

##### 5E: Capability System ✅
- [x] **CAP-001**: Capability enum (36 capabilities)
- [x] **CAP-002**: CapabilitySet bitmap (64-bit)
- [x] **CAP-003**: Per-process capability sets (effective)
- [x] **CAP-004**: Capability inheritance on fork()
- [x] **CAP-005**: Predefined sets (all, empty, user, admin, network)
- [x] **CAP-006**: Capability checking in security module
- [x] **CAP-007**: Linux-compatible + DebOS extension capabilities
- [x] **CAP-008**: Root capability bypass (CAP_DAC_OVERRIDE)

##### 5F: Superuser & Privilege Escalation ✅
- [x] **ROOT-001**: Root (UID 0) detection and handling
- [x] **ROOT-002**: Root login disabled by default
- [x] **ROOT-003**: su command implementation
- [x] **ROOT-004**: sudo command implementation
- [x] **ROOT-005**: Admin check (wheel group membership)
- [x] **ROOT-006**: Privilege elevation for su/sudo
- [x] **ROOT-007**: Capability-based privilege (admin_default)

##### 5G: Security Policies ✅
- [x] **POL-001**: ResourceLimits struct (max procs, files, memory)
- [x] **POL-002**: Role-based limits (Admin, User, Service, Guest)
- [x] **POL-003**: SecurityPolicy struct (configurable)
- [x] **POL-004**: RBAC policy enforcement
- [x] **POL-005**: Audit event logging (in-memory)
- [x] **POL-006**: Policy actions (Login, Sudo, ManageUsers)
- [x] **POL-007**: Configurable lockout and password policies

##### 5H: User Management Commands ✅
- [x] **CMD-001**: useradd command (with -a admin flag, -p password)
- [x] **CMD-002**: userdel command
- [x] **CMD-003**: passwd command (change password)
- [x] **CMD-004**: id command (show user/group info)
- [x] **CMD-005**: whoami command
- [x] **CMD-006**: users command (list all users)
- [x] **CMD-007**: groups command (list all groups)
- [x] **CMD-008**: su command (switch user)
- [x] **CMD-009**: sudo command (run as admin with audit)
- [x] **CMD-010**: login command
- [x] **CMD-011**: logout command

### Priority 5: Standard Library & Tooling

- [ ] **LIB-001**: libdebos syscall wrappers
- [ ] **LIB-002**: libdebos thread API
- [ ] **LIB-003**: libdebos filesystem API
- [ ] **LIB-004**: libdebos networking API
- [ ] **TOOL-001**: Disk image creation scripts
- [ ] **TOOL-002**: initramfs builder
- [ ] **TOOL-003**: CI/CD pipeline

---

## Testing Milestones

| Phase | Test | Expected Result |
|-------|------|-----------------|
| 1 | Dual thread VGA output | Two threads printing to console simultaneously |
| 2 | ext4 file read | Mount disk image, read file content |
| 2 | ICMP ping | `ping 8.8.8.8` returns responses |
| 3 | AI calculator | Text prompt generates calculator UI |
| 4 | Green thread spawn | 10 million threads spawned < 10 seconds |
| 4 | Context switch benchmark | < 100ns average switch time |
| 4 | Work stealing | Linear speedup with core count |
| 4 | Async I/O | 1M IOPS with green threads |
| 5 | User isolation | Process A (uid=1000) cannot access Process B (uid=1001) files |
| 5 | Authentication | Invalid passwords rejected, valid accepted |
| 5 | File permissions | Permission denied for unauthorized file access |
| 5 | Capability system | Non-root with CAP_NET_BIND_SERVICE can bind port 80 |
| 5 | Sudo/su | Privilege escalation works with correct password |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| IPC performance bottleneck | Medium | High | Implement direct switch optimization early |
| ONNX Runtime porting complexity | High | Medium | Consider lighter ML frameworks as fallback |
| ext4 driver complexity | Medium | Medium | Start with FAT32, add ext4 incrementally |
| Real-time scheduler starvation | Low | High | Careful priority assignment, aging for normal class |

---

## References

- L4 Microkernel Family (IPC design inspiration)
- seL4 (Capability-based security model)
- Redox OS (Rust microkernel reference)
- ONNX Runtime (AI inference)

---

*This document will be updated as implementation progresses.*
