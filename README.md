# DebOS

> A POSIX-compatible microkernel operating system written in Rust with AI integration capabilities.

## Overview

DebOS is a modern operating system built on a microkernel architecture (DeK - DebOS Nano-Kernel). It provides:

- **Microkernel Design**: Superior security and stability through minimal kernel code
- **Memory Safety**: Built with Rust's `no_std` for guaranteed memory safety
- **AI-First**: Intent Engine and Generative UI as first-class citizens
- **Capability-Based Security**: Fine-grained access control for all resources
- **Multi-Architecture**: Supports x86_64 and AArch64 (Apple Silicon)

## Architecture

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

## Supported Architectures

| Architecture | Target | Status | Notes |
|--------------|--------|--------|-------|
| x86_64 | `x86_64-unknown-none` | ✅ Ready | Intel/AMD processors |
| AArch64 | `aarch64-unknown-none` | ✅ Ready | Apple Silicon (M1/M2/M3), ARM64 |

## Quick Start

### Prerequisites

**macOS (Apple Silicon):**
```bash
# Install Homebrew if not installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install qemu nasm

# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup default nightly
rustup component add rust-src llvm-tools-preview
rustup target add x86_64-unknown-none aarch64-unknown-none
```

**Linux (x86_64 or ARM64):**
```bash
# Debian/Ubuntu
sudo apt-get update
sudo apt-get install qemu-system-x86 qemu-system-arm nasm

# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup default nightly
rustup component add rust-src llvm-tools-preview
rustup target add x86_64-unknown-none aarch64-unknown-none
```

### Building

```bash
# Clone the repository
git clone https://github.com/your-org/debos.git
cd debos

# Build for your native architecture (auto-detected)
make build

# Or explicitly:
make build-arm    # For Apple Silicon / AArch64
make build-x86    # For Intel/AMD / x86_64

# Check both architectures compile
make check
```

### Running in QEMU

```bash
# Run for your native architecture (auto-detected)
make run

# Or explicitly:
make run-arm      # Run AArch64 kernel in QEMU
make run-x86      # Run x86_64 kernel in QEMU
```

### Using Docker (Recommended for CI/CD)

```bash
# Build the Docker image
make docker-build

# Run the kernel in Docker
make docker-run-arm    # For AArch64
make docker-run-x86    # For x86_64
```

## Project Structure

```
debos/
├── kernel/               # DeK - DebOS Nano-Kernel
│   ├── src/
│   │   ├── arch/         # Architecture-specific code
│   │   │   ├── x86_64/   # Intel/AMD support
│   │   │   └── aarch64/  # ARM64/Apple Silicon support
│   │   ├── memory/       # Memory management
│   │   ├── scheduler/    # Thread scheduling
│   │   ├── ipc/          # Inter-process communication
│   │   ├── syscall/      # System call interface
│   │   ├── shell/        # Built-in kernel shell
│   │   └── capability/   # Capability-based security
│   ├── linker.ld         # x86_64 linker script
│   └── linker-aarch64.ld # AArch64 linker script
├── libdebos/             # User-space standard library
├── servers/              # Core userspace servers
│   ├── vfs/              # Virtual Filesystem Server
│   ├── netserver/        # Networking Server
│   ├── devman/           # Device Manager
│   └── intent_engine/    # AI Intent Engine
├── drivers/              # Userspace drivers
│   ├── virtio_block/     # VirtIO Block Driver
│   └── virtio_net/       # VirtIO Network Driver
├── genshell/             # Generative UI Shell
├── Dockerfile            # Build environment
├── Makefile              # Build commands
└── rust-toolchain.toml   # Rust version specification
```

## Interactive Shell

DebOS includes a built-in kernel shell for system interaction:

```
debos> help
debos> info          # System information
debos> mem           # Memory statistics
debos> threads       # List threads
debos> uptime        # Show uptime

# Filesystem commands (RamFS)
debos> pwd           # Print working directory
debos> ls            # List directory
debos> mkdir test    # Create directory
debos> cat file.txt  # Read file

# Block device commands (VirtIO)
debos> disk          # Show block device info
debos> mount         # Mount FAT32 filesystem
debos> fatls /       # List FAT32 directory
debos> fatcat file   # Read FAT32 file

debos> clear         # Clear screen
debos> exit          # Exit shell
```

## Testing with Disk Images

DebOS supports VirtIO block devices and FAT32 filesystems:

```bash
# Create a test disk with FAT32 (requires mtools)
brew install mtools  # macOS
make new-disk        # Creates test_disk.img (16 MB FAT32)

# Add files to the disk
echo "Hello DebOS!" > /tmp/hello.txt
mcopy -i test_disk.img /tmp/hello.txt ::HELLO.TXT

# Run with the disk attached
make run-arm-disk

# In the DebOS shell:
debos> disk          # Shows: VirtIO-Block: 32768 sectors
debos> mount         # Mounts FAT32
debos> fatls /       # Lists files
debos> fatcat hello.txt  # Reads file content
```

## ⚠️ Storage Safety

**Your MacBook's storage is completely protected during development:**

| Layer | Protection |
|-------|------------|
| **QEMU Virtualization** | Complete hardware isolation - guest OS cannot access host devices |
| **File-Backed Disks** | Only uses `.img` files, never raw block devices (`/dev/disk*`) |
| **Makefile Guards** | All disk commands explicitly use file-backed images only |
| **No Root Required** | Development runs entirely in userspace |

The VirtIO block driver in DebOS can only access the virtual disk image provided by QEMU. There is no code path that could access your actual Mac storage.

```bash
# These are the ONLY disk operations used:
-drive file=test_disk.img,format=raw,if=none,id=hd0  # File-backed only!
```

## Development on Apple Silicon Mac

DebOS is fully compatible with Apple Silicon Macs (M1, M2, M3). Here's the recommended workflow:

### Native AArch64 Development (Fastest)

When developing on Apple Silicon, build and test with the AArch64 target for best performance:

```bash
# Build for AArch64
make build-arm

# Run in QEMU with hardware virtualization (fast!)
make run-arm
```

QEMU will use Apple's Hypervisor.framework for near-native performance.

### Cross-Compiling for x86_64

You can also build and test the x86_64 version on Apple Silicon:

```bash
# Build for x86_64
make build-x86

# Run in QEMU (uses emulation, slower)
make run-x86
```

Note: x86_64 emulation on Apple Silicon is functional but slower than native AArch64.

## Architecture Details

### x86_64 Specifics
- Uses BIOS/UEFI boot via `bootloader` crate
- 8259 PIC for interrupt handling
- Serial output via COM1 (16550 UART)
- 4-level paging (PML4)

### AArch64 Specifics
- Direct kernel boot (suitable for QEMU virt machine)
- GICv2 for interrupt handling
- Serial output via PL011 UART
- 4-level paging (compatible layout)
- ARM architectural timer for preemption

## Development Status

### Phase 1: Kernel Parity ✅ Complete
- [x] Project structure setup
- [x] GDT/IDT initialization (x86_64)
- [x] Exception/GIC handling (AArch64)
- [x] Memory management (buddy allocator, heap)
- [x] Thread scheduler (O(1) priority-based)
- [x] Context switching (both architectures)
- [x] IPC primitives with direct switch optimization
- [x] System call interface (x86_64 syscall, AArch64 SVC)
- [x] Capability system
- [x] Interactive kernel shell

### Phase 2: Core Drivers 🔄 In Progress
- [x] In-kernel RamFS filesystem
- [x] VirtIO subsystem (MMIO transport)
- [x] VirtIO-Block driver
- [x] FAT32 filesystem (read support)
- [ ] VirtIO-Net driver
- [ ] VFS Server (userspace)
- [ ] Network Server (userspace)

### Phase 3: AI Layer (Planned)
- [ ] Intent Engine
- [ ] Generative UI (GenShell)
- [ ] ONNX Runtime integration

### Phase 4: Advanced Concurrency (Planned)
- [ ] Green threading (M:N model)
- [ ] Work-stealing scheduler
- [ ] Async I/O subsystem
- [ ] GPU compute integration (opt-in)

## Contributing

Contributions are welcome! Please ensure your code:

1. Compiles for both x86_64 and AArch64 (`make check`)
2. Follows Rust formatting (`make fmt`)
3. Includes appropriate documentation

## License

MIT OR Apache-2.0
