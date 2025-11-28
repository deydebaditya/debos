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

#### Phase 2D: Userspace Servers
- [ ] **SRV-001**: VFS Server skeleton with IPC listener
- [ ] **SRV-002**: VFS protocol implementation
- [ ] **SRV-003**: Server-based FAT32 driver
- [ ] **SRV-004**: ext4 filesystem driver
- [ ] **SRV-005**: NetServer skeleton
- [ ] **SRV-006**: TCP/IP stack (lwIP port or custom)
- [ ] **SRV-007**: Device Manager with PCI enumeration
- [ ] **SRV-008**: Driver loading mechanism

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
