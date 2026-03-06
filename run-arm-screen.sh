#!/bin/bash
# Alternative: Use screen to create a PTY session for QEMU
# This sometimes works better on macOS

set -e

# Build the kernel first
echo "Building kernel..."
make build-arm > /dev/null 2>&1

echo "=========================================="
echo "Running QEMU via screen (PTY)"
echo "=========================================="
echo "Press Ctrl+A then K to kill screen and exit"
echo ""

# Check if screen is available
if ! command -v screen &> /dev/null; then
    echo "ERROR: screen not found. Install with: brew install screen"
    echo "Falling back to stdio method..."
    exec ./run-arm-input.sh
    exit 1
fi

# Create a screen session with QEMU
# Screen creates a PTY which QEMU can use
exec screen -S debos-qemu \
  qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a72 \
    -m 512M \
    -nographic \
    -kernel target/aarch64-unknown-none/release/debos-kernel \
    -serial stdio \
    -monitor none

