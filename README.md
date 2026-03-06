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

---

## Prerequisites & Installation

### macOS (Apple Silicon / Intel)

#### 1. Install Homebrew

```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Add Homebrew to PATH (Apple Silicon)
echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
eval "$(/opt/homebrew/bin/brew shellenv)"
```

#### 2. Install Build Dependencies

```bash
# Install QEMU and NASM
brew install qemu nasm

# For FAT32 disk image creation (optional)
brew install mtools
```

#### 3. Install Rust Toolchain

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain
rustup toolchain install nightly
rustup default nightly

# Add required components
rustup component add rust-src llvm-tools-preview

# Add cross-compilation targets
rustup target add x86_64-unknown-none
rustup target add aarch64-unknown-none
```

#### 4. Verify Installation

```bash
# Check Rust version (should be nightly)
rustc --version

# Check QEMU
qemu-system-aarch64 --version
qemu-system-x86_64 --version
```

---

### Linux (Ubuntu/Debian)

#### 1. Update Package Manager

```bash
sudo apt-get update
sudo apt-get upgrade -y
```

#### 2. Install Build Dependencies

```bash
# Install QEMU, NASM, and build tools
sudo apt-get install -y \
    qemu-system-x86 \
    qemu-system-arm \
    nasm \
    build-essential \
    curl \
    git

# For FAT32 disk image creation (optional)
sudo apt-get install -y mtools
```

#### 3. Install Rust Toolchain

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install nightly toolchain
rustup toolchain install nightly
rustup default nightly

# Add required components
rustup component add rust-src llvm-tools-preview

# Add cross-compilation targets
rustup target add x86_64-unknown-none
rustup target add aarch64-unknown-none
```

#### 4. Verify Installation

```bash
# Check Rust version
rustc --version

# Check QEMU
qemu-system-x86_64 --version
qemu-system-aarch64 --version
```

---

### Linux (Fedora/RHEL/CentOS)

#### 1. Install Build Dependencies

```bash
# Install QEMU, NASM, and build tools
sudo dnf install -y \
    qemu-system-x86 \
    qemu-system-aarch64 \
    nasm \
    gcc \
    make \
    curl \
    git

# For FAT32 disk image creation (optional)
sudo dnf install -y mtools
```

#### 2. Install Rust Toolchain

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install nightly toolchain
rustup toolchain install nightly
rustup default nightly

# Add required components
rustup component add rust-src llvm-tools-preview

# Add cross-compilation targets
rustup target add x86_64-unknown-none
rustup target add aarch64-unknown-none
```

---

### Windows (WSL2 Recommended)

**Note:** Native Windows support is limited. We recommend using WSL2 (Windows Subsystem for Linux) for the best experience.

#### 1. Install WSL2

```powershell
# Run PowerShell as Administrator
wsl --install

# Restart your computer when prompted
```

#### 2. Install Ubuntu in WSL2

After restart, Ubuntu will launch automatically. Follow the Linux (Ubuntu/Debian) instructions above.

#### 3. Alternative: Native Windows (Advanced)

If you must use native Windows:

```powershell
# Install Rust via rustup
# Download and run: https://rustup.rs/

# Install QEMU
# Download from: https://www.qemu.org/download/#windows
# Or use Chocolatey:
choco install qemu

# Install NASM
choco install nasm

# Add Rust targets
rustup target add x86_64-unknown-none
rustup target add aarch64-unknown-none
```

**Note:** Native Windows builds may have limitations. WSL2 is strongly recommended.

---

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/deydebaditya/debos.git
cd debos
```

### 2. Build the Kernel

```bash
# Build for your native architecture (auto-detected)
make build

# Or explicitly:
make build-arm    # For Apple Silicon / AArch64
make build-x86    # For Intel/AMD / x86_64

# Build both architectures
make check
```

### 3. Run in QEMU

```bash
# Run for your native architecture (auto-detected)
make run

# Or explicitly:
make run-arm      # Run AArch64 kernel in QEMU
make run-x86      # Run x86_64 kernel in QEMU
```

### 4. Exit QEMU

- Type `shutdown` or `poweroff` in the DebOS shell to power off (terminates QEMU)
- Type `reboot` in the DebOS shell to restart
- Press `Ctrl+A` then `X` to force-quit QEMU
- Type `exit` in the DebOS shell to exit the shell (kernel continues running)

---

## Building with Disk Images

### Create a Test Disk (FAT32)

```bash
# macOS / Linux
make new-disk      # Creates test_disk.img (16 MB FAT32)

# Or manually:
dd if=/dev/zero of=test_disk.img bs=1M count=16
mformat -i test_disk.img -F ::
```

### Add Files to Disk

```bash
# Create a test file
echo "Hello DebOS!" > /tmp/hello.txt

# Copy to FAT32 disk (macOS/Linux)
mcopy -i test_disk.img /tmp/hello.txt ::HELLO.TXT

# List files on disk
mdir -i test_disk.img
```

### Run with Disk Attached

```bash
# Run with VirtIO block device
make run-arm-disk  # AArch64 with disk
make run-x86-disk  # x86_64 with disk
```

### Use in DebOS Shell

```
debos (/)> disk          # Shows: VirtIO-Block: 32768 sectors
debos (/)> mount         # Mounts FAT32 filesystem
debos (/)> fatls /       # Lists files
debos (/)> fatcat HELLO.TXT  # Reads file content
```

---

## Interactive Shell

DebOS includes a built-in kernel shell with 40+ commands. The prompt shows the
current user and working directory, and updates automatically after `su`,
`login`, or `sudo`:

```
debos (/)> su megha
Switched to user: megha
megha (/)> cd /home
megha (/home)>
```

### System Commands
```
debos> help          # Show all commands
debos> info          # System information
debos> mem           # Memory statistics
debos> ps            # List threads
debos> uptime        # Show uptime
debos> clear         # Clear screen
debos> shutdown      # Power off the system (terminates QEMU)
debos> reboot        # Reboot the system (restarts QEMU)
```

### Filesystem Commands (RamFS)
```
debos> pwd           # Print working directory
debos> ls            # List directory
debos> mkdir test    # Create directory
debos> touch file.txt # Create file
debos> cat file.txt  # Read file
debos> rm file.txt   # Remove file
debos> stat file.txt # File metadata
```

### Block Device Commands (FAT32)
```
debos> disk          # Show block device info
debos> mount         # Mount FAT32 filesystem
debos> fatls /       # List FAT32 directory
debos> fatcat file   # Read FAT32 file
debos> fatwrite file "text"  # Write to FAT32 file
debos> fatrm file    # Delete FAT32 file
```

### User & Security Commands
```
debos> whoami        # Current user
debos> id            # User/group IDs
debos> users         # List all users
debos> groups        # List all groups
debos> useradd name  # Create user
debos> passwd        # Change password
debos> su user       # Switch user
debos> sudo cmd      # Run as admin
```

### Network Commands
```
debos> ifconfig      # Network interfaces
debos> ping 8.8.8.8  # Ping host
debos> arp           # ARP cache
debos> netstat       # Network statistics
```

### Device Commands
```
debos> devices       # List all devices
debos> lspci         # List PCI devices
debos> lsusb         # List USB devices
```

---

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
│   │   ├── fs/           # Filesystem (RamFS, FAT32, ext4)
│   │   ├── drivers/      # Device drivers
│   │   └── security/     # User management & security
│   ├── linker.ld         # x86_64 linker script
│   └── linker-aarch64.ld # AArch64 linker script
├── libdebos/             # User-space standard library
├── servers/               # Core userspace servers
│   ├── vfs/              # Virtual Filesystem Server
│   ├── netserver/        # Networking Server
│   ├── devman/           # Device Manager
│   └── intent_engine/    # AI Intent Engine
├── docs/                 # Documentation
│   └── developer/        # Developer documentation
├── Dockerfile            # Build environment
├── Makefile             # Build commands
└── rust-toolchain.toml  # Rust version specification
```

---

## ⚠️ Storage Safety

**Your host machine's storage is completely protected during development:**

| Layer | Protection |
|-------|------------|
| **QEMU Virtualization** | Complete hardware isolation - guest OS cannot access host devices |
| **File-Backed Disks** | Only uses `.img` files, never raw block devices (`/dev/disk*`, `\\.\PhysicalDrive*`) |
| **Makefile Guards** | All disk commands explicitly use file-backed images only |
| **No Root Required** | Development runs entirely in userspace |

The VirtIO block driver in DebOS can only access the virtual disk image provided by QEMU. There is no code path that could access your actual storage.

```bash
# These are the ONLY disk operations used:
-drive file=test_disk.img,format=raw,if=none,id=hd0  # File-backed only!
```

---

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

---

## Architecture Details

### x86_64 Specifics
- Uses BIOS/UEFI boot via `bootloader` crate
- 8259 PIC for interrupt handling
- Serial output via COM1 (16550 UART)
- 4-level paging (PML4)

### AArch64 Specifics
- Direct kernel boot (suitable for QEMU virt machine)
- GICv2 for interrupt handling (timer + UART IRQs)
- Serial I/O via PL011 UART (polling + interrupt-driven RX)
- 4-level paging (compatible layout)
- ARM architectural timer for preemption
- PSCI support for `shutdown` and `reboot` (via HVC)

---

## Troubleshooting

### Build Issues

**Error: `cargo: command not found`**
```bash
# Ensure Rust is installed and in PATH
source $HOME/.cargo/env  # Linux/macOS
# Or restart your terminal
```

**Error: `linker 'cc' not found`**
```bash
# Install build tools
# macOS:
xcode-select --install

# Ubuntu/Debian:
sudo apt-get install build-essential

# Fedora:
sudo dnf install gcc
```

**Error: `rustup: command not found`**
```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Runtime Issues

**QEMU not found:**
```bash
# macOS:
brew install qemu

# Ubuntu/Debian:
sudo apt-get install qemu-system-x86 qemu-system-arm

# Fedora:
sudo dnf install qemu-system-x86 qemu-system-aarch64
```

**QEMU fails to start:**
- Ensure virtualization is enabled in BIOS/UEFI
- On macOS, ensure Hypervisor.framework is available
- Check QEMU version: `qemu-system-aarch64 --version`

**Kernel doesn't boot:**
- Check that the kernel binary exists: `ls target/aarch64-unknown-none/release/debos-kernel`
- Verify QEMU command in Makefile
- Try running QEMU manually to see error messages

**Disk image not found:**
```bash
# Create test disk
make new-disk

# Or manually:
dd if=/dev/zero of=test_disk.img bs=1M count=16
mformat -i test_disk.img -F ::
```

---

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

### Phase 2: Core Drivers ✅ Complete
- [x] In-kernel RamFS filesystem
- [x] VirtIO subsystem (MMIO transport)
- [x] VirtIO-Block driver
- [x] FAT32 filesystem (read/write support)
- [x] ext4 filesystem (read support)
- [x] Device Manager core
- [x] USB subsystem (xHCI, HID, Mass Storage)
- [x] Network stack (Ethernet → TCP/IP)
- [x] Display subsystem (VirtIO-GPU, framebuffer)
- [x] VFS Server (userspace with IPC)

### Phase 5: User Management & Security ✅ Complete
- [x] User and group management
- [x] Argon2id password hashing
- [x] Process credentials
- [x] File permissions (POSIX-style)
- [x] Capability system
- [x] Authentication and session management

### Phase 3: AI Layer (Planned)
- [ ] Intent Engine
- [ ] Generative UI (GenShell)
- [ ] ONNX Runtime integration

### Phase 4: Advanced Concurrency (Planned)
- [ ] Green threading (M:N model)
- [ ] Work-stealing scheduler
- [ ] Async I/O subsystem
- [ ] GPU compute integration (opt-in)

---

## Contributing

Contributions are welcome! Please ensure your code:

1. Compiles for both x86_64 and AArch64 (`make check`)
2. Follows Rust formatting (`make fmt`)
3. Includes appropriate documentation
4. Passes all tests

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

---

## Running in VirtualBox

DebOS can be run in VirtualBox for a more traditional VM experience. See the [VirtualBox Setup Guide](docs/VIRTUALBOX_SETUP.md) for detailed instructions.

**Quick Start:**
```bash
# Build kernel and create bootable ISO
make build-x86
make iso-x86

# Then create a VM in VirtualBox and boot from debos.iso
```

**Note:** VirtualBox requires x86_64 builds. AArch64 builds only work in QEMU.

## Resources

- [Changelog](CHANGELOG.md) - Release history and notable changes
- [Implementation Plan](IMPLEMENTATION_PLAN.md) - Detailed development roadmap
- [Test Report](TEST_REPORT.md) - Comprehensive test results
- [VirtualBox Setup Guide](docs/VIRTUALBOX_SETUP.md) - Running DebOS in VirtualBox
- [Developer Documentation](docs/developer/) - Technical deep-dives
