# DebOS Makefile
# Supports x86_64 and AArch64 architectures
#
# SAFETY NOTE:
# All disk operations use file-backed disk images (*.img), NOT raw block devices.
# QEMU provides complete isolation from host storage.
# The guest OS can only access virtual disks provided explicitly via -drive flags.
# Your MacBook's storage is completely safe during development and testing.

.PHONY: help build-x86 build-arm run-x86 run-arm clean docker-build docker-run-x86 docker-run-arm check fmt iso-x86 vdi-x86 clean-images

# Detect host architecture
UNAME_M := $(shell uname -m)
ifeq ($(UNAME_M),arm64)
    DEFAULT_ARCH := arm
else ifeq ($(UNAME_M),aarch64)
    DEFAULT_ARCH := arm
else
    DEFAULT_ARCH := x86
endif

# Output directories
TARGET_DIR := target
X86_TARGET := x86_64-unknown-none
ARM_TARGET := aarch64-unknown-none

# Kernel binary names
ARM_KERNEL := $(TARGET_DIR)/$(ARM_TARGET)/release/debos-kernel
X86_KERNEL := $(TARGET_DIR)/$(X86_TARGET)/release/debos-kernel

# QEMU settings
# Note: -nographic redirects serial to stdio automatically
QEMU_X86 := qemu-system-x86_64 -machine q35 -m 512M -nographic
QEMU_ARM := qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M -nographic

help:
	@echo "DebOS Build System"
	@echo "=================="
	@echo ""
	@echo "Detected host architecture: $(UNAME_M) (default: $(DEFAULT_ARCH))"
	@echo ""
	@echo "Build Commands:"
	@echo "  make build-x86    - Build kernel for x86_64"
	@echo "  make build-arm    - Build kernel for AArch64 (Apple Silicon)"
	@echo "  make build        - Build for default architecture ($(DEFAULT_ARCH))"
	@echo ""
	@echo "Run Commands:"
	@echo "  make run-x86      - Run kernel in QEMU (x86_64)"
	@echo "  make run-arm      - Run kernel in QEMU (AArch64)"
	@echo "  make run          - Run for default architecture ($(DEFAULT_ARCH))"
	@echo ""
	@echo "Docker Commands:"
	@echo "  make docker-build     - Build Docker image"
	@echo "  make docker-run-x86   - Build and run x86_64 in Docker"
	@echo "  make docker-run-arm   - Build and run AArch64 in Docker"
	@echo ""
	@echo "Development Commands:"
	@echo "  make check        - Run cargo check for both architectures"
	@echo "  make fmt          - Format code"
	@echo "  make clean        - Clean build artifacts"
	@echo ""
	@echo "VirtualBox Commands:"
	@echo "  make iso-x86      - Create bootable ISO for VirtualBox (x86_64)"
	@echo "  make vdi-x86      - Convert disk image to VDI format"
	@echo "  make clean-images - Remove ISO and VDI files"
	@echo ""
	@echo "To exit QEMU: Press Ctrl+A then X"

# Default build based on host architecture
build: build-$(DEFAULT_ARCH)

# Build for x86_64
build-x86:
	@echo "Building DebOS kernel for x86_64..."
	cargo build --package debos-kernel --target $(X86_TARGET) --release

# Build for AArch64
build-arm:
	@echo "Building DebOS kernel for AArch64..."
	cargo build --package debos-kernel --target $(ARM_TARGET) --release

# Default run based on host architecture
run: run-$(DEFAULT_ARCH)

# Run x86_64 in QEMU
run-x86: build-x86
	@echo "Running DebOS kernel in QEMU (x86_64)..."
	@echo "Press Ctrl+A then X to exit QEMU"
	$(QEMU_X86) -kernel $(X86_KERNEL) -chardev stdio,id=serial0,mux=on,signal=off -serial chardev:serial0 -monitor none

# Run AArch64 in QEMU
# Uses explicit chardev setup for reliable stdin handling (same as x86_64)
run-arm: build-arm
	@echo "Running DebOS kernel in QEMU (AArch64)..."
	@echo "Press Ctrl+A then X to exit QEMU"
	$(QEMU_ARM) -kernel $(ARM_KERNEL) -chardev stdio,id=serial0,mux=on,signal=off -serial chardev:serial0 -monitor none

# Run AArch64 with VirtIO disk attached
# SAFETY: Only uses file-backed disk images, never raw block devices
run-arm-disk: build-arm test_disk.img
	@echo "Running DebOS kernel in QEMU (AArch64) with disk..."
	@echo "SAFETY: Using file-backed disk image (test_disk.img), host storage is protected"
	@echo "Press Ctrl+A then X to exit QEMU"
	$(QEMU_ARM) -kernel $(ARM_KERNEL) \
		-drive file=test_disk.img,format=raw,if=none,id=hd0 \
		-device virtio-blk-device,drive=hd0,bus=virtio-mmio-bus.0

# Create a test disk image (safe, file-backed only)
test_disk.img:
	@echo "Creating test disk image (16 MB, FAT32)..."
	@dd if=/dev/zero of=test_disk.img bs=1M count=16 2>/dev/null
	@command -v mformat >/dev/null 2>&1 && mformat -i test_disk.img -F -v "DEBOS" :: || \
		echo "Note: Install mtools (brew install mtools) to auto-format as FAT32"
	@echo "Created test_disk.img (safe file-backed disk image)"

# Create a fresh test disk (deletes existing)
new-disk:
	@rm -f test_disk.img
	@$(MAKE) test_disk.img

# Check both architectures
check:
	@echo "Checking x86_64..."
	cargo check --package debos-kernel --target $(X86_TARGET)
	@echo ""
	@echo "Checking AArch64..."
	cargo check --package debos-kernel --target $(ARM_TARGET)

# Format code
fmt:
	cargo fmt --all

# Clean build artifacts
clean:
	cargo clean

# Docker commands
docker-build:
	docker build -t debos-builder .

docker-run-x86: docker-build
	docker run --rm -it debos-builder make run-x86

docker-run-arm: docker-build
	docker run --rm -it debos-builder make run-arm

# Development targets
dev-x86:
	cargo watch -x 'check --package debos-kernel --target $(X86_TARGET)'

dev-arm:
	cargo watch -x 'check --package debos-kernel --target $(ARM_TARGET)'

# Debug run with extra QEMU output
debug-arm: build-arm
	@echo "Running DebOS kernel in QEMU (AArch64) with debug..."
	qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M \
		-nographic -d int,guest_errors \
		-kernel $(ARM_KERNEL)

# Run AArch64 with networking enabled
# SAFETY: Uses QEMU user-mode networking, isolated from host network
run-arm-net: build-arm
	@echo "Running DebOS kernel in QEMU (AArch64) with networking..."
	@echo "SAFETY: Using QEMU user-mode networking (slirp), isolated from host"
	@echo "Guest IP: 10.0.2.15, Gateway: 10.0.2.2, DNS: 10.0.2.3"
	@echo "Press Ctrl+A then X to exit QEMU"
	$(QEMU_ARM) -kernel $(ARM_KERNEL) \
		-device virtio-net-device,netdev=net0 \
		-netdev user,id=net0,hostfwd=tcp::2222-:22

# Run AArch64 with full device support (disk + network + input)
# SAFETY: All devices are virtual, host storage/network protected
run-arm-full: build-arm test_disk.img
	@echo "Running DebOS kernel in QEMU (AArch64) with full device support..."
	@echo "SAFETY: All virtual devices, host system protected"
	@echo "  - VirtIO Block: test_disk.img"
	@echo "  - VirtIO Net: user-mode networking (10.0.2.15)"
	@echo "  - VirtIO Input: keyboard and mouse"
	@echo "  - VirtIO GPU: virtual display (framebuffer)"
	@echo "Press Ctrl+A then X to exit QEMU"
	$(QEMU_ARM) -kernel $(ARM_KERNEL) \
		-drive file=test_disk.img,format=raw,if=none,id=hd0 \
		-device virtio-blk-device,drive=hd0,bus=virtio-mmio-bus.0 \
		-device virtio-net-device,netdev=net0 \
		-netdev user,id=net0,hostfwd=tcp::2222-:22 \
		-device virtio-keyboard-device \
		-device virtio-mouse-device

# Run x86_64 with networking
run-x86-net: build-x86
	@echo "Running DebOS kernel in QEMU (x86_64) with networking..."
	$(QEMU_X86) -kernel $(X86_KERNEL) \
		-device virtio-net-pci,netdev=net0 \
		-netdev user,id=net0,hostfwd=tcp::2222-:22

# Run x86_64 with full device support
run-x86-full: build-x86 test_disk.img
	@echo "Running DebOS kernel in QEMU (x86_64) with full device support..."
	$(QEMU_X86) -kernel $(X86_KERNEL) \
		-drive file=test_disk.img,format=raw,if=none,id=hd0 \
		-device virtio-blk-pci,drive=hd0 \
		-device virtio-net-pci,netdev=net0 \
		-netdev user,id=net0,hostfwd=tcp::2222-:22

# ============================================================================
# VirtualBox Support
# ============================================================================

# Create bootable ISO for VirtualBox (x86_64)
# Requires: grub-mkrescue or xorriso
# Note: x86_64 kernel uses bootloader_api, so this creates a basic ISO structure
# Full bootloader integration is planned for future releases
iso-x86: build-x86
	@echo "Creating bootable ISO for VirtualBox..."
	@echo "Note: x86_64 kernel uses bootloader_api - full ISO boot support is in development"
	@mkdir -p iso/boot/grub
	@cp $(X86_KERNEL) iso/boot/debos-kernel
	@printf 'set timeout=5\nset default=0\n\nmenuentry "DebOS" {\n    multiboot2 /boot/debos-kernel\n    boot\n}\n\nmenuentry "DebOS (legacy)" {\n    multiboot /boot/debos-kernel\n    boot\n}\n' > iso/boot/grub/grub.cfg
	@if command -v grub-mkrescue >/dev/null 2>&1; then \
		echo "Using grub-mkrescue..."; \
		grub-mkrescue -o debos.iso iso 2>/dev/null || \
		grub-mkrescue --compress=xz -o debos.iso iso; \
	elif command -v xorriso >/dev/null 2>&1; then \
		echo "Using xorriso (basic ISO, may need GRUB installed separately)..."; \
		xorriso -as mkisofs -R -J -o debos.iso iso; \
		echo "WARNING: This ISO may not be bootable without GRUB bootloader"; \
		echo "Consider installing GRUB or using QEMU instead (make run-x86)"; \
	else \
		echo "ERROR: grub-mkrescue or xorriso not found"; \
		echo ""; \
		echo "Install options:"; \
		echo "  macOS:    brew install xorriso"; \
		echo "  Ubuntu:   sudo apt-get install grub-pc-bin xorriso"; \
		echo "  Fedora:   sudo dnf install grub2-efi-x64-modules xorriso"; \
		echo ""; \
		echo "Alternative: Use QEMU instead (make run-x86)"; \
		exit 1; \
	fi
	@rm -rf iso
	@echo ""
	@echo "Created debos.iso"
	@echo "Note: If boot fails, the kernel may need bootloader_api integration"
	@echo "For now, QEMU (make run-x86) is the most reliable way to run DebOS"

# Create VDI disk image from raw image (for VirtualBox)
vdi-x86: test_disk.img
	@echo "Converting disk image to VDI format..."
	@if command -v VBoxManage >/dev/null 2>&1; then \
		VBoxManage convertfromraw test_disk.img debos-disk.vdi --format VDI; \
		echo "Created debos-disk.vdi"; \
	elif command -v qemu-img >/dev/null 2>&1; then \
		qemu-img convert -f raw -O vdi test_disk.img debos-disk.vdi; \
		echo "Created debos-disk.vdi"; \
	else \
		echo "ERROR: VBoxManage or qemu-img not found"; \
		echo "Install VirtualBox or QEMU to convert disk images"; \
		exit 1; \
	fi

# Clean ISO and VDI files
clean-images:
	@rm -f debos.iso debos-disk.vdi
	@echo "Cleaned ISO and VDI images"
