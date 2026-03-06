# Shell Input Fix Summary

**Status: RESOLVED**

## Problem Statement
The shell prompt appears but keyboard input doesn't work - the cursor is stuck and typing doesn't produce any output in QEMU. Characters typed appear in the Mac terminal when QEMU is killed, proving that QEMU is not capturing stdin at all.

## Timeline of Fixes Attempted

### 1. Initial Issue: Shell Not Starting
**Problem**: Shell thread was spawned but didn't start running, no prompt appeared.

**Root Cause**: 
- Scheduler only started on first timer interrupt, causing delay
- Scheduler lock was held during context switch, causing deadlock when shell thread tried to access credentials

**Fixes Applied**:
- Created `start_scheduler()` function that releases lock before context switching
- Updated `start_shell()` to call `start_scheduler()` immediately instead of waiting for timer interrupt
- Updated `on_timer_tick()` to use `start_scheduler()` for consistency

**Files Modified**:
- `kernel/src/scheduler/mod.rs`: Added `start_scheduler()` function
- `kernel/src/lib.rs`: Updated `start_shell()` to call `scheduler::start_scheduler()`

**Result**: ✅ Shell prompt now appears

---

### 2. Issue: Input Not Working (Cursor Stuck)
**Problem**: Shell prompt appears but keyboard input doesn't work - cursor is stuck.

**Root Causes Identified**:
1. Context switch doesn't restore SPSR register, so interrupts might not be enabled
2. Tight polling loop prevents interrupts from being processed
3. QEMU serial configuration might not be forwarding input correctly

**Fixes Attempted**:

#### Fix 2.1: Context Switch Interrupt Enable
- **Files Modified**: `kernel/src/arch/aarch64/context.rs`
- **Changes**:
  - Added SPSR save in `context_switch` (offset 14*8 = 112 bytes)
  - Added SPSR restore in `context_switch_first`
  - Added explicit interrupt enable with `msr DAIFClr, #0xf`
- **Result**: ❌ Still not working

#### Fix 2.2: Polling Strategy Changes
- **Files Modified**: `kernel/src/shell/mod.rs`
- **Changes**:
  - Changed from busy-wait loop to `wait_for_interrupt()` + `yield_now()`
  - Later changed to hybrid approach: frequent polling with occasional yields and sleeps
  - Reduced delay from 100 iterations to 10 iterations for faster polling
- **Result**: ❌ Still not working

#### Fix 2.3: QEMU Configuration Changes
- **Files Modified**: `Makefile`, `run-arm-input.sh`
- **Changes**:
  - Tried `-serial stdio` with `-nographic`
  - Tried `-chardev stdio,id=uart0 -serial chardev:uart0`
  - Tried `-display none` instead of `-nographic`
  - Created wrapper script with `stty raw -echo -icanon min 1 time 0`
- **Result**: ❌ Still not working - QEMU doesn't capture stdin even in raw mode

#### Fix 2.4: Alternative Approaches
- **Files Created**: `run-arm-screen.sh`, `run-arm-pty.sh`
- **Approach**: Use PTY (pseudo-terminal) via `screen` or `socat` as intermediary
- **Result**: ❌ Not tested - user reported still not working

---

## Current Status — RESOLVED

All issues have been fixed. The shell is fully interactive on both macOS (local)
and Linux (RPi / Docker sandbox).

### What Works:
- ✅ Shell thread starts and runs
- ✅ Shell prompt appears correctly with dynamic username
- ✅ Keyboard input works (typing, backspace, Enter)
- ✅ Output (println!) works correctly
- ✅ Interrupts are enabled (timer + UART interrupts fire correctly)
- ✅ Timer interrupt re-arms properly (no IRQ storm)
- ✅ Scheduler works correctly
- ✅ UART driver polls RX FIFO and handles interrupts
- ✅ VFS uses SDK for deadlock-free credential access
- ✅ SDK uses try_lock() to prevent deadlocks
- ✅ Key repeat throttling (3s hold delay, 500ms repeat)
- ✅ `shutdown` and `reboot` commands work via PSCI
- ✅ No double prompts (handles `\r\n` correctly)

---

## Root Cause Analysis

The issue is **definitely QEMU configuration** rather than kernel code:
- The kernel code is correctly polling the UART
- The UART driver correctly checks the RX FIFO empty flag
- But QEMU isn't forwarding stdin to the virtual UART device

**Evidence**:
- Characters appear in Mac terminal when QEMU is killed → Terminal IS sending them
- QEMU never receives them → QEMU process isn't reading from stdin
- Even raw mode doesn't help → Suggests QEMU process itself has an issue

**Possible causes**:
1. **QEMU 10.1.2 on macOS bug**: There may be a known issue with stdin forwarding on macOS
2. **Terminal emulator interference**: iTerm might be interfering with stdin forwarding
3. **QEMU virt machine serial configuration**: The virt machine might need explicit UART device creation
4. **macOS terminal handling**: macOS might require special handling for QEMU stdin

---

## Files Modified Summary

### Kernel Code:
- `kernel/src/scheduler/mod.rs`: Added `start_scheduler()` function, fixed deadlock, added `try_current_credentials()` with try_lock()
- `kernel/src/lib.rs`: Updated shell startup to call `start_scheduler()`
- `kernel/src/arch/aarch64/context.rs`: Added SPSR save/restore and interrupt enable
- `kernel/src/shell/mod.rs`: Updated polling strategy, removed debug output
- `kernel/src/shell/input.rs`: Improved UART read with error handling
- `kernel/src/shell/sdk.rs`: Created SDK module for safe credential access using try_lock()
- `kernel/src/shell/commands.rs`: Updated to use SDK functions
- `kernel/src/fs/vfs.rs`: Now uses SDK's `get_owner_ids()` for deadlock-free credential access
- `kernel/src/arch/aarch64/uart.rs`: Improved read_byte() with error handling, added has_data()

### Build System & Scripts:
- `Makefile`: Updated QEMU to use `-chardev stdio,id=serial0,mux=on,signal=off -serial chardev:serial0` (same as x86_64)
- `run-arm-input.sh`: Updated to use explicit chardev configuration with raw terminal mode
- `run-arm-screen.sh`: Created alternative script using screen PTY
- `run-arm-pty.sh`: Created alternative script using socat PTY
- `test-qemu-input.sh`: Created diagnostic script

---

## Lessons Learned

1. **Bare-metal echo tests** were the key diagnostic — they proved QEMU *does*
   deliver stdin to PL011, isolating the problem to the kernel's interrupt
   handling rather than QEMU configuration.
2. **Binary search through init stages** (placing the echo test progressively
   deeper into kernel init) efficiently pinpointed the timer interrupt as the
   culprit.
3. **Timer re-arming is mandatory** on AArch64 — unlike x86's APIC timer in
   periodic mode, the ARM generic timer's `ISTATUS` must be cleared by
   reloading the countdown register.

---

## Testing Commands

```bash
# Build
make build-arm

# Run with explicit chardev configuration (matches x86_64 setup)
make run-arm

# Try raw terminal mode (recommended on macOS)
./run-arm-input.sh

# Try screen PTY (alternative)
./run-arm-screen.sh

# Try socat PTY (requires: brew install socat)
./run-arm-pty.sh

# Check QEMU version
qemu-system-aarch64 --version

# Manual test with verbose QEMU debug output
qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M \
  -nographic -d int,guest_errors \
  -chardev stdio,id=serial0,mux=on,signal=off \
  -serial chardev:serial0 -monitor none \
  -kernel target/aarch64-unknown-none/release/debos-kernel
```

---

## Resolution

The input issue was ultimately caused by **three separate bugs** that were fixed
incrementally:

### Bug 1: UART `read_byte()` error check (commit `d640815`)
The PL011 driver checked `FR[3]` (BUSY bit) instead of `DR[11:8]` (error bits).
Any byte received while the UART was transmitting (e.g. printing the prompt)
was silently discarded.

### Bug 2: QEMU chardev mux conflict (commit `a6c4020`)
The Makefile used `-chardev stdio,mux=on` alongside `-nographic`. These flags
conflict — `-nographic` already connects serial to stdio. Removing the explicit
chardev and mux flags resolved the conflict.

### Bug 3: Timer interrupt storm (commit `6134f77`) — ROOT CAUSE
The AArch64 timer interrupt handler (`on_timer_tick`) did not reload
`CNTP_TVAL_EL0` after handling the interrupt. The timer's `ISTATUS` flag
remained set, causing the IRQ to fire continuously in an infinite loop. This
starved the shell thread of all CPU time, making input impossible.

**Fix**: Re-arm the timer by writing a new countdown value to `CNTP_TVAL_EL0`
at the end of the interrupt handler.

### Additional improvements after resolution:
- Key repeat throttling with gap detection (commits `1c13f94`, `8ead00e`)
- Dynamic username in prompt (commit `aa90820`)
- `\r\n` double-prompt fix (commit `8d0d25d`)
- `shutdown`/`reboot` via PSCI (commit `8d0d25d`)

**Status**: ✅ Fully resolved
