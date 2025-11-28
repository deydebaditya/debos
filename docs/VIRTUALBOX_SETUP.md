# Running DebOS in VirtualBox

This guide explains how to run DebOS in VirtualBox on Windows, macOS, and Linux.

> **⚠️ Important Note:** DebOS's x86_64 kernel currently uses `bootloader_api` which requires proper bootloader integration. Full VirtualBox support is in development. For the most reliable experience, **QEMU is recommended** (`make run-x86`). This guide provides workarounds for VirtualBox users.

## Prerequisites

1. **VirtualBox** (version 6.0 or later)
   - Download from: https://www.virtualbox.org/wiki/Downloads
   - Install VirtualBox Extension Pack for full USB support

2. **Build Tools** (to create bootable image)
   - `xorriso` or `genisoimage` - for creating ISO images
   - `grub` or `grub-pc-bin` - for bootloader (Linux)
   - Or use the provided Makefile targets

## Method 1: Bootable ISO Image (Experimental)

**Status:** ⚠️ Experimental - x86_64 kernel uses `bootloader_api` which may require additional bootloader setup.

This method creates a GRUB-based ISO, but full compatibility depends on bootloader integration.

### Prerequisites for ISO Creation

**macOS:**
```bash
brew install xorriso
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt-get install grub-pc-bin xorriso
```

**Linux (Fedora/RHEL):**
```bash
sudo dnf install grub2-efi-x64-modules xorriso
```

### Step 1: Build the Kernel

```bash
# Build for x86_64 (VirtualBox requires x86_64)
make build-x86
```

### Step 2: Create Bootable ISO

```bash
# Create ISO with GRUB bootloader
make iso-x86
```

This will create `debos.iso` in the project root.

**Note:** The x86_64 kernel uses `bootloader_api` which requires a bootloader. The ISO creation uses GRUB multiboot2 to boot the kernel.

### Step 3: Create VirtualBox VM

1. **Open VirtualBox** and click "New"

2. **VM Settings:**
   - **Name:** DebOS
   - **Type:** Linux
   - **Version:** Other Linux (64-bit)
   - **Memory:** 512 MB (minimum), 1 GB recommended
   - **Hard Disk:** Create a virtual hard disk now
     - **File size:** 2 GB (minimum)
     - **Hard disk file type:** VDI (VirtualBox Disk Image)
     - **Storage on physical hard disk:** Dynamically allocated

3. **Configure VM:**
   - Right-click the VM → Settings
   - **System → Motherboard:**
     - Enable I/O APIC
     - Enable EFI (optional, for UEFI boot)
   - **System → Processor:**
     - Enable PAE/NX (if available)
     - Processors: 1-2 cores
   - **Storage:**
     - Click "Empty" under Controller: IDE
     - Click the disk icon → Choose Virtual Optical Disk File
     - Select `debos.iso`
   - **Network:**
     - Adapter 1: NAT (default) or Bridged Adapter
   - **Serial Ports:**
     - Port 1: Enable, Port Mode: Host Pipe, Path: `\\.\pipe\debos` (Windows) or `/tmp/debos` (Linux/macOS)

4. **Start the VM:**
   - Select the VM and click "Start"
   - DebOS should boot from the ISO

## Method 2: Using QEMU Disk Image in VirtualBox (Simpler)

Since VirtualBox doesn't support direct kernel boot like QEMU, you can convert QEMU's disk images:

```bash
# Create a disk image with the kernel
make test_disk.img

# Convert to VDI format
make vdi-x86

# Or manually:
VBoxManage convertfromraw test_disk.img debos-disk.vdi --format VDI
```

Then attach `debos-disk.vdi` as a hard disk in VirtualBox.

**Note:** This requires the kernel to be installed on the disk, which is not yet automated.

## Method 3: Direct Kernel Boot with GRUB (Advanced)

If you have GRUB installed, you can create a custom bootable disk:

### Step 1: Create Disk Image

```bash
# Create a 2GB disk image
dd if=/dev/zero of=debos-disk.img bs=1M count=2048

# Partition the disk (optional, for full installation)
# Use fdisk or parted to create partitions
```

### Step 2: Install GRUB

```bash
# Mount the disk image
sudo losetup -P /dev/loop0 debos-disk.img
sudo mkfs.ext4 /dev/loop0p1  # If partitioned

# Mount the partition
sudo mount /dev/loop0p1 /mnt

# Install GRUB
sudo grub-install --target=x86_64-efi --boot-directory=/mnt/boot --efi-directory=/mnt/boot/efi /dev/loop0

# Create GRUB configuration
sudo mkdir -p /mnt/boot/grub
cat <<EOF | sudo tee /mnt/boot/grub/grub.cfg
menuentry "DebOS" {
    multiboot2 /boot/debos-kernel
    boot
}
EOF

# Copy kernel
sudo cp target/x86_64-unknown-none/release/debos-kernel /mnt/boot/

# Unmount
sudo umount /mnt
sudo losetup -d /dev/loop0
```

### Step 3: Use in VirtualBox

1. Create VM as above
2. Instead of ISO, attach `debos-disk.img` as a hard disk
3. Boot the VM


## Troubleshooting

### VM Won't Boot

**Issue:** Black screen or "No bootable medium found"
- **Solution:** 
  - Ensure ISO is attached in Storage settings
  - Check that ISO was created correctly: `file debos.iso`
  - Try enabling/disabling EFI in VM settings

### Kernel Panic on Boot

**Issue:** Kernel crashes immediately
- **Solution:**
  - Increase VM memory to at least 512 MB
  - Check serial port output for error messages
  - Ensure kernel was built for x86_64: `file target/x86_64-unknown-none/release/debos-kernel`

### No Serial Output

**Issue:** Can't see kernel messages
- **Solution:**
  - Enable serial port in VM settings
  - Use VirtualBox's serial console or connect via pipe
  - On Windows: Use PuTTY to connect to `\\.\pipe\debos`
  - On Linux/macOS: `socat - UNIX-CONNECT:/tmp/debos`

### Slow Performance

**Issue:** VM runs very slowly
- **Solution:**
  - Enable hardware virtualization in BIOS/UEFI
  - Enable VT-x/AMD-V in VM settings (System → Acceleration)
  - Allocate more CPU cores
  - Use SSD for VM disk storage

### Network Not Working

**Issue:** No network connectivity
- **Solution:**
  - Ensure network adapter is enabled
  - Use NAT mode for basic connectivity
  - Check that VirtIO-Net driver is loaded (if implemented)
  - Verify network stack initialization in kernel

## VirtualBox-Specific Features

### Shared Folders

To share files between host and guest:

1. Install VirtualBox Guest Additions (when supported)
2. Or use network file sharing
3. Or mount host directory via network

### USB Passthrough

To use USB devices in DebOS:

1. Install VirtualBox Extension Pack
2. Enable USB controller in VM settings
3. Add USB device filters for your devices
4. Ensure DebOS USB drivers are loaded

### Snapshots

Take VM snapshots before major changes:

1. VM → Take Snapshot
2. Name: "Clean DebOS Install"
3. Restore if needed: VM → Snapshots → Restore

## Performance Tips

1. **Allocate sufficient resources:**
   - RAM: 512 MB minimum, 1-2 GB recommended
   - CPU: 1-2 cores
   - Disk: 2 GB minimum, 10 GB for development

2. **Enable hardware acceleration:**
   - System → Acceleration → Enable VT-x/AMD-V
   - Enable Nested Paging

3. **Use SSD storage:**
   - Store VM disk on SSD for better I/O performance

4. **Disable unnecessary features:**
   - Audio (if not needed)
   - USB (if not using USB devices)
   - 3D acceleration (not needed for text console)

## Method 4: Direct Kernel Boot (QEMU-style, Not Recommended for VirtualBox)

**Note:** VirtualBox doesn't support direct kernel boot like QEMU. Use ISO method instead.

If you want to test quickly without creating an ISO, use QEMU:

```bash
# Run directly in QEMU (cross-platform)
make run-x86

# Or with full device support
make run-x86-full
```

QEMU provides better compatibility with DebOS's current boot setup and supports direct kernel boot.

## Current Limitations & Workarounds

### x86_64 Boot Requirements

The x86_64 kernel uses `bootloader_api` which means:
- It requires a bootloader that supports `bootloader_api`
- Cannot boot directly as a raw kernel binary
- Full bootloader integration is planned but not yet complete

### Recommended Workaround: Use QEMU

**For the most reliable experience, use QEMU instead of VirtualBox:**

```bash
# Run directly in QEMU (works immediately)
make run-x86

# Or with full device support
make run-x86-full
```

QEMU supports direct kernel boot (`-kernel` flag) which works perfectly with DebOS's current setup.

### AArch64 Support

- VirtualBox does **not** support AArch64/ARM64
- AArch64 builds only work in QEMU
- Use `make run-arm` for AArch64 testing

### Future: Full VirtualBox Support

Planned improvements:
- [ ] Complete bootloader integration for x86_64
- [ ] Automated ISO creation with proper GRUB setup
- [ ] UEFI boot support
- [ ] Automated VM creation scripts

## Recommended: Use QEMU Instead

**QEMU is the recommended virtualization solution for DebOS** because:

1. ✅ **Direct kernel boot** - No bootloader setup needed
2. ✅ **Cross-platform** - Works on Windows, macOS, Linux
3. ✅ **Better compatibility** - Designed for kernel development
4. ✅ **Full device support** - VirtIO devices work out of the box
5. ✅ **Simpler setup** - Just run `make run-x86`

```bash
# Run directly in QEMU (cross-platform)
make run-x86

# Or with full device support
make run-x86-full
```

QEMU provides better compatibility with DebOS's current boot setup and is the primary development target.

## Next Steps

Once DebOS is running in VirtualBox:

1. **Test the shell:** Type `help` to see available commands
2. **Explore filesystem:** Use `ls`, `mkdir`, `cat` commands
3. **Test networking:** Use `ifconfig`, `ping` (when network is configured)
4. **Mount disk:** Use `mount` to access FAT32 disk images

## Notes

- **Architecture:** VirtualBox requires x86_64. AArch64 builds won't work in VirtualBox.
- **Boot Method:** Currently uses direct kernel boot. Full bootloader support is planned.
- **Storage Safety:** VirtualBox provides complete isolation - your host storage is safe.

---

For issues or questions, check the [Test Report](TEST_REPORT.md) or open an issue on GitHub.

