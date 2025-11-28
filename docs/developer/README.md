# DebOS Developer Documentation

Welcome to the DebOS developer documentation. This folder contains technical guides and implementation plans for DebOS subsystems.

## Documentation Index

| Document | Description | Status |
|----------|-------------|--------|
| [FILESYSTEM_IMPLEMENTATION.md](./FILESYSTEM_IMPLEMENTATION.md) | Filesystem & VFS implementation plan | 📋 Planned |
| NETWORKING_IMPLEMENTATION.md | TCP/IP stack implementation | 🔜 Coming Soon |
| IPC_DEEP_DIVE.md | IPC internals and optimization | 🔜 Coming Soon |
| DRIVER_DEVELOPMENT.md | How to write DebOS drivers | 🔜 Coming Soon |

## Quick Status

### Implemented ✅

| Component | Location | Notes |
|-----------|----------|-------|
| Kernel Boot (x86_64 & AArch64) | `kernel/src/main.rs` | Both architectures working |
| Memory Management | `kernel/src/memory/` | Buddy allocator + heap |
| Thread Scheduler | `kernel/src/scheduler/` | O(1) priority scheduler |
| IPC Primitives | `kernel/src/ipc/` | RPC-style communication |
| Kernel Shell | `kernel/src/shell/` | Basic interactive shell |
| Serial/UART Output | `kernel/src/arch/*/` | Console I/O working |

### Not Yet Implemented ❌

| Component | Location | Priority |
|-----------|----------|----------|
| Filesystem (VFS) | `servers/vfs/` | **Phase 2A** - Next |
| Block Devices | `drivers/virtio_block/` | Phase 2C |
| Networking | `servers/netserver/` | Phase 2D |
| FAT32/ext4 | `servers/vfs/` | Phase 2C |
| Intent Engine | `servers/intent_engine/` | Phase 3 |
| GenShell UI | `genshell/` | Phase 3 |

## Architecture Decisions

### Why Microkernel?

DebOS uses a microkernel architecture where:
- **Kernel (DeK)** handles only: scheduling, IPC, memory management, interrupts
- **Everything else** runs in userspace servers (filesystem, networking, drivers)

Benefits:
- 🔒 Better security isolation
- 🛡️ Fault containment (driver crash doesn't crash kernel)
- 🔧 Easier to develop and test components
- 📦 Modular and replaceable services

### Current Compromise: In-Kernel Bootstrap

For Phase 2A (filesystem), we're implementing an in-kernel RamFS first:
- Allows shell commands to work immediately
- No need for userspace server infrastructure yet
- Will be migrated to VFS server in Phase 2B

## Getting Started (Development)

### Prerequisites

```bash
# macOS (Apple Silicon)
brew install qemu nasm
rustup default nightly
rustup target add aarch64-unknown-none x86_64-unknown-none
rustup component add rust-src llvm-tools-preview
```

### Build & Run

```bash
# Build for your architecture
make build          # Auto-detects (arm on Apple Silicon)

# Run in QEMU
make run            # Auto-detects

# Explicit architecture
make build-arm      # AArch64
make run-arm        # Run AArch64 in QEMU
```

### Project Structure

```
debos/
├── kernel/                 # DeK - DebOS Nano-Kernel
│   └── src/
│       ├── arch/           # Architecture-specific code
│       │   ├── x86_64/     # Intel/AMD support
│       │   └── aarch64/    # ARM64/Apple Silicon
│       ├── memory/         # Memory management
│       ├── scheduler/      # Thread scheduling
│       ├── ipc/            # Inter-process communication
│       ├── syscall/        # System call handlers
│       ├── shell/          # Kernel shell
│       ├── capability/     # Capability system
│       └── fs/             # [PLANNED] In-kernel filesystem
├── libdebos/               # Userspace standard library
├── servers/                # Userspace servers
│   ├── vfs/                # [STUB] Virtual Filesystem
│   ├── netserver/          # [STUB] Networking
│   ├── devman/             # [STUB] Device Manager
│   └── intent_engine/      # [STUB] AI Intent Engine
├── drivers/                # Userspace drivers
│   ├── virtio_block/       # [STUB] VirtIO Block
│   └── virtio_net/         # [STUB] VirtIO Network
├── genshell/               # [STUB] Generative UI Shell
└── docs/developer/         # This documentation
```

## Contributing

1. Check the implementation plan for the component you want to work on
2. Follow the existing code style and patterns
3. Test on both x86_64 and AArch64 if possible
4. Update documentation as you implement

## Contact

For questions about DebOS development, check the [IMPLEMENTATION_PLAN.md](../../IMPLEMENTATION_PLAN.md) in the project root.

