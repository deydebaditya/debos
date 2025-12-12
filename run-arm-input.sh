#!/bin/bash
# Wrapper script to run QEMU with proper terminal setup for input on macOS
# This script sets the terminal to raw mode which is REQUIRED for QEMU to capture stdin

set -e

# Build the kernel first
echo "Building kernel..."
make build-arm > /dev/null 2>&1

echo "=========================================="
echo "Running QEMU with RAW TERMINAL MODE"
echo "=========================================="
echo "Press Ctrl+A then X to exit QEMU"
echo ""
echo "IMPORTANT: Terminal is now in raw mode."
echo "All input will go directly to QEMU."
echo ""

# Save current terminal settings
OLD_STTY=$(stty -g 2>/dev/null || echo "")

# Trap to ensure terminal is restored on exit
cleanup() {
    if [ -n "$OLD_STTY" ]; then
        stty "$OLD_STTY" 2>/dev/null || stty sane
    else
        stty sane
    fi
    echo ""  # Newline after QEMU exits
}
trap cleanup EXIT INT TERM

# CRITICAL: Set terminal to raw mode BEFORE running QEMU
# Without this, QEMU cannot read from stdin on macOS
# The order matters: set raw mode, THEN exec QEMU
# Try multiple stty commands to ensure it works
stty raw -echo -icanon min 1 time 0 2>/dev/null || \
stty raw -echo 2>/dev/null || \
stty raw 2>/dev/null || true

# Verify raw mode is set
if ! stty -a 2>/dev/null | grep -q "raw"; then
    echo "WARNING: Failed to set terminal to raw mode"
    echo "Input may not work properly"
fi

# Run QEMU with -serial stdio
# The virt machine automatically creates PL011 UART at 0x0900_0000
# and connects it to the first serial port
# With -nographic, -serial stdio connects serial0 to stdin/stdout
# NOTE: On macOS, QEMU MUST be run with terminal in raw mode for stdin to work
exec qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a72 \
  -m 512M \
  -nographic \
  -kernel target/aarch64-unknown-none/release/debos-kernel \
  -serial stdio \
  -monitor none
