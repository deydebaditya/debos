#!/bin/bash
# Test script to run QEMU with proper terminal setup for input

echo "Testing QEMU input with different configurations..."
echo ""

# Test 1: Basic -nographic with -serial stdio
echo "Test 1: -nographic -serial stdio"
qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M \
  -nographic \
  -kernel target/aarch64-unknown-none/release/debos-kernel \
  -serial stdio \
  -monitor none

echo ""
echo "If input didn't work, try Test 2..."

