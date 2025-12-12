#!/bin/bash
# Alternative approach: Use a PTY for QEMU serial connection
# This sometimes works better on macOS than direct stdio

set -e

# Build the kernel first
echo "Building kernel..."
make build-arm > /dev/null 2>&1

echo "=========================================="
echo "Running QEMU with PTY-based serial"
echo "=========================================="
echo "Press Ctrl+A then X to exit QEMU"
echo ""

# Create a PTY and get its name
PTY=$(socat -d -d pty,raw,echo=0 pty,raw,echo=0 2>&1 | grep -oE '/dev/ttys?[0-9]+' | head -1)

if [ -z "$PTY" ]; then
    echo "ERROR: socat not found. Install with: brew install socat"
    echo "Falling back to stdio method..."
    exec ./run-arm-input.sh
    exit 1
fi

echo "PTY created: $PTY"
echo "Connecting QEMU to PTY..."
echo ""

# Run socat in background to connect PTY to stdio
socat -d -d "$PTY",raw,echo=0 stdio &
SOCAT_PID=$!

# Cleanup function
cleanup() {
    kill $SOCAT_PID 2>/dev/null || true
    wait $SOCAT_PID 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# Small delay for socat to set up
sleep 0.5

# Run QEMU connected to the PTY
exec qemu-system-aarch64 \
  -machine virt \
  -cpu cortex-a72 \
  -m 512M \
  -nographic \
  -kernel target/aarch64-unknown-none/release/debos-kernel \
  -chardev pty,id=uart0 \
  -serial chardev:uart0 \
  -monitor none

