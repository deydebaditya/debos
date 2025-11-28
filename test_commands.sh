#!/bin/bash
# Test script for DebOS features

# Start QEMU with test commands via stdin
cd /Users/deba/Work/gitRepos/aegis-os

echo "Starting DebOS testing..."

# Create a FIFO for communication
rm -f /tmp/debos_test_in /tmp/debos_test_out
mkfifo /tmp/debos_test_in

# Start QEMU in background with input from FIFO
(cat /tmp/debos_test_in | timeout 60 qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a72 \
    -m 512M \
    -nographic \
    -kernel target/aarch64-unknown-none/release/debos-kernel 2>&1) &

QEMU_PID=$!
sleep 5

# Function to send command and wait
send_cmd() {
    echo "$1" > /tmp/debos_test_in
    sleep 1
}

# Test commands
echo "======= TESTING SECURITY SUBSYSTEM =======" 
send_cmd "whoami"
send_cmd "id"
send_cmd "users"
send_cmd "groups"

echo "======= TESTING USER MANAGEMENT ======="
send_cmd "useradd testuser"
send_cmd "users"

echo "======= TESTING FILESYSTEM ======="
send_cmd "ls"
send_cmd "mkdir /testdir"
send_cmd "ls"
send_cmd "cd /testdir"
send_cmd "pwd"
send_cmd "cd /"

echo "======= TESTING SHELL UTILITIES ======="
send_cmd "echo test content > /testfile.txt"
send_cmd "cat /testfile.txt"
send_cmd "head /testfile.txt"

echo "======= TESTING SYSTEM INFO ======="
send_cmd "mem"
send_cmd "threads"
send_cmd "help"

echo "======= CLEANUP ======="
send_cmd "exit"

# Wait a bit then kill QEMU
sleep 2
kill $QEMU_PID 2>/dev/null

# Cleanup
rm -f /tmp/debos_test_in /tmp/debos_test_out

echo "Testing complete!"

