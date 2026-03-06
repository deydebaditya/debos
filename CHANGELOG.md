# Changelog

All notable changes to DebOS are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.2.0] - 2026-03-06

### Added

- **Dynamic shell prompt**: The prompt now displays the currently logged-in
  user's name and working directory (e.g. `megha (/home)>`). Username is
  resolved on every prompt render so it updates immediately after `su`, `login`,
  or `sudo`.
- **`shutdown` / `poweroff` command**: Powers off the system cleanly using
  PSCI `SYSTEM_OFF` (AArch64) or ACPI PM1a (x86_64). Terminates the QEMU
  emulator on exit.
- **`reboot` command (AArch64)**: Now functional on AArch64 via PSCI
  `SYSTEM_RESET`. Previously printed a "not implemented" stub.
- **Key repeat throttling**: The shell distinguishes between a key being held
  down (terminal auto-repeat) and deliberate re-presses. Holding a key
  suppresses repeats for 3 seconds, then allows one repeat every 500 ms. Lifting
  and re-pressing the same key is detected via a 200 ms gap and accepted
  immediately.
- **Interrupt-driven UART RX**: Added a lock-free ring buffer and
  `handle_rx_interrupt()` for PL011. The GIC now dispatches UART IRQ 33 to
  drain the FIFO into the buffer.

### Fixed

- **Shell input not working on AArch64**: Root cause was a timer interrupt
  storm — `on_timer_tick()` did not re-arm `CNTP_TVAL_EL0`, so the timer IRQ
  fired continuously and starved the shell thread. Fixed by re-arming the
  timer in the interrupt handler.
- **Double prompt on Enter**: Terminals send `\r\n` for a single Enter press.
  The shell now tracks a `skip_lf` flag so the trailing `\n` after `\r` is
  consumed silently instead of producing a second empty prompt.
- **UART `read_byte()` discarding valid input**: The PL011 driver was checking
  the Flag Register's BUSY bit (`FR[3]`) instead of the Data Register's error
  bits (`DR[11:8]`). Valid bytes received while the UART was transmitting were
  incorrectly discarded.
- **QEMU chardev mux conflict**: The `run-arm` Makefile target used
  `-chardev stdio,mux=on` alongside `-nographic`, which conflicted. Simplified
  to rely on `-nographic` alone.
- **`module sdk is private`**: Changed `mod sdk` to `pub(crate) mod sdk` in
  `kernel/src/shell/mod.rs` so that `vfs.rs` can access SDK credential helpers.
- **Scheduler deadlock on credential access**: Added `try_current_credentials()`
  using `try_lock()` to prevent deadlocks when the scheduler lock is already
  held.

### Changed

- **Shell idle loop**: Replaced `wfi` (Wait For Interrupt) with a bounded
  `spin_loop()` in the shell's `read_line()` to ensure QEMU's event loop gets
  CPU time for stdin processing.
- **PL011 UART init**: No longer disables and re-enables the UART during
  `init()`, preserving QEMU's default chardev connection.
- **PL011 FIFO**: Disabled the FIFO (`LCR_H = 0x60`) so every character
  generates an interrupt, improving compatibility with QEMU TCG mode.

---

## [0.1.0] - Initial Release

### Features

- **Microkernel architecture** (DeK — DebOS Nano-Kernel)
- **Dual architecture support**: x86_64 and AArch64
- **Memory management**: Buddy allocator, heap allocator, 4-level paging
- **O(1) priority-based thread scheduler** with preemptive multitasking
- **IPC primitives** with direct-switch optimisation
- **System call interface**: `syscall` (x86_64), `svc` (AArch64)
- **Interactive kernel shell** with 40+ commands
- **In-kernel RamFS** filesystem with POSIX-style permissions
- **VirtIO subsystem** (MMIO transport) with VirtIO-Block driver
- **FAT32 filesystem** (read/write) and **ext4** (read-only)
- **USB subsystem**: xHCI, HID, Mass Storage
- **Network stack**: Ethernet, ARP, IPv4, ICMP, UDP, TCP
- **Display subsystem**: VirtIO-GPU, framebuffer
- **User management**: Users, groups, Argon2id password hashing
- **Capability-based security** with process credentials
- **VFS Server** (userspace, IPC-based)
