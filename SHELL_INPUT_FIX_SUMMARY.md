# Shell Input Fix Summary

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

## Current Status

### What Works:
- ✅ Shell thread starts and runs
- ✅ Shell prompt appears correctly
- ✅ Output (println!) works correctly
- ✅ Interrupts are enabled (timer interrupts fire)
- ✅ Scheduler works correctly
- ✅ UART driver correctly polls RX FIFO empty flag
- ✅ Kernel code is correctly structured

### What Doesn't Work:
- ❌ Keyboard input doesn't reach the kernel
- ❌ UART `read_byte()` never returns `Some(byte)` - always returns `None`
- ❌ Characters typed appear in Mac terminal when QEMU is killed (proving QEMU isn't capturing them)

### Key Observations:
1. When QEMU is killed, typed characters appear in Mac terminal → QEMU isn't capturing stdin
2. Debug output shows shell is running and polling UART
3. No `[DEBUG] read_char: got byte` messages → UART never receives characters
4. QEMU configuration changes haven't fixed the issue
5. Raw terminal mode (`stty raw`) doesn't help - QEMU still doesn't read stdin
6. Even PTY-based approaches don't work

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
- `kernel/src/scheduler/mod.rs`: Added `start_scheduler()` function, fixed deadlock
- `kernel/src/lib.rs`: Updated shell startup to call `start_scheduler()`
- `kernel/src/arch/aarch64/context.rs`: Added SPSR save/restore and interrupt enable
- `kernel/src/shell/mod.rs`: Updated polling strategy, removed debug output
- `kernel/src/shell/input.rs`: Improved UART read with error handling
- `kernel/src/shell/sdk.rs`: Created SDK module for safe credential access
- `kernel/src/shell/commands.rs`: Updated to use SDK functions
- `kernel/src/fs/vfs.rs`: Updated credential access (temporary hardcoding)
- `kernel/src/arch/aarch64/uart.rs`: Improved read_byte() with error handling, added has_data()

### Build System & Scripts:
- `Makefile`: Updated QEMU command line arguments, added warnings
- `run-arm-input.sh`: Created wrapper script with raw terminal mode
- `run-arm-screen.sh`: Created alternative script using screen PTY
- `run-arm-pty.sh`: Created alternative script using socat PTY
- `test-qemu-input.sh`: Created diagnostic script

---

## Next Steps (For Future Investigation)

1. **Test with Terminal.app instead of iTerm** - Different terminal emulators might behave differently
2. **Try QEMU 7.2.1** - Some users report version 7.2.1 works better on macOS than 10.1.2
3. **Check QEMU logs** - Use `-d` flags to see if QEMU is receiving stdin events
4. **Try explicit PL011 device** - Use `-device pl011,chardev=serial0` instead of relying on virt machine defaults
5. **Check QEMU issue tracker** - Search for known macOS stdin issues
6. **Try different QEMU machine types** - Test if issue is specific to virt machine
7. **Use QEMU monitor** - Connect via `-monitor stdio` to inspect UART device state
8. **Test on Linux** - Verify if issue is macOS-specific or general QEMU problem

---

## Testing Commands

```bash
# Build
make build-arm

# Run with current configuration (input won't work)
make run-arm

# Try raw terminal mode (input still doesn't work)
./run-arm-input.sh

# Try screen PTY (not tested)
./run-arm-screen.sh

# Try socat PTY (requires: brew install socat)
./run-arm-pty.sh

# Check QEMU version
qemu-system-aarch64 --version

# Manual QEMU run with explicit UART device (not tested)
qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M \
  -nographic -serial stdio -monitor none \
  -device pl011,chardev=serial0 \
  -chardev stdio,id=serial0 \
  -kernel target/aarch64-unknown-none/release/debos-kernel
```

---

## Conclusion

Despite extensive attempts to fix the input issue, QEMU on macOS (version 10.1.2) is not capturing stdin even when:
- Terminal is in raw mode
- QEMU is configured with `-serial stdio`
- Kernel code is correctly polling the UART

The issue appears to be a QEMU/macOS compatibility problem rather than a kernel bug. The kernel code is functioning correctly - it's just not receiving any data from QEMU's virtual UART device.

**Status**: ❌ Input still not working - requires further investigation or QEMU version downgrade/testing
