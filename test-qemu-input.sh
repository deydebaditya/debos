#!/bin/bash
# Test script to verify QEMU can receive stdin

echo "Testing if QEMU can receive stdin..."
echo ""

# Test 1: Simple echo test
echo "Test 1: Direct stdin to QEMU"
echo "hello" | qemu-system-aarch64 -machine virt -cpu cortex-a72 -m 512M \
  -nographic -serial stdio -monitor none \
  -kernel target/aarch64-unknown-none/release/debos-kernel 2>&1 | head -5 || true

echo ""
echo "If you see 'hello' in QEMU output, stdin is working"
echo ""

