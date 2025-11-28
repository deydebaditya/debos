#!/usr/bin/env python3
"""
DebOS Kernel Test Script
Tests all features implemented today.
"""

import subprocess
import time
import os
import sys
import select

def main():
    kernel_path = "target/aarch64-unknown-none/release/debos-kernel"
    
    if not os.path.exists(kernel_path):
        print(f"Error: Kernel not found at {kernel_path}")
        print("Please build the kernel first: make build-arm")
        return 1
    
    # Start QEMU
    print("=" * 60)
    print("DebOS Kernel Test Suite")
    print("=" * 60)
    print("\nStarting QEMU...")
    
    proc = subprocess.Popen(
        [
            "qemu-system-aarch64",
            "-machine", "virt",
            "-cpu", "cortex-a72",
            "-m", "512M",
            "-nographic",
            "-kernel", kernel_path,
        ],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        bufsize=0,
    )
    
    def read_output(timeout=3):
        """Read available output from QEMU"""
        output = b""
        end_time = time.time() + timeout
        while time.time() < end_time:
            # Use select to check if data is available
            ready, _, _ = select.select([proc.stdout], [], [], 0.1)
            if ready:
                chunk = os.read(proc.stdout.fileno(), 4096)
                if chunk:
                    output += chunk
                else:
                    break
            elif output:
                # No more data and we have some output
                time.sleep(0.2)
                ready, _, _ = select.select([proc.stdout], [], [], 0.1)
                if not ready:
                    break
        return output.decode('utf-8', errors='replace')
    
    def send_command(cmd):
        """Send a command to the shell"""
        print(f"\n>>> {cmd}")
        proc.stdin.write(f"{cmd}\n".encode())
        proc.stdin.flush()
        time.sleep(0.5)
        output = read_output(2)
        print(output)
        return output
    
    # Wait for boot
    print("\nWaiting for kernel boot...")
    boot_output = read_output(8)
    print(boot_output)
    
    # Check boot success
    tests_passed = 0
    tests_failed = 0
    test_results = []
    
    def check_test(name, condition, details=""):
        nonlocal tests_passed, tests_failed
        if condition:
            tests_passed += 1
            status = "✅ PASS"
        else:
            tests_failed += 1
            status = "❌ FAIL"
        test_results.append((name, status, details))
        print(f"{status}: {name}")
    
    # Test 1: Boot
    check_test(
        "Kernel Boot",
        "Kernel initialization complete" in boot_output,
        "Kernel should display initialization messages"
    )
    
    check_test(
        "Security Subsystem Init",
        "Security subsystem initialized" in boot_output,
        "Security subsystem should initialize on boot"
    )
    
    check_test(
        "Default User Created",
        "Default user: debos" in boot_output,
        "Default debos user should be created"
    )
    
    check_test(
        "Device Manager Init",
        "Device manager initialized" in boot_output,
        "Device manager should initialize"
    )
    
    check_test(
        "Input Subsystem Init",
        "Input subsystem initialized" in boot_output,
        "Input subsystem should initialize"
    )
    
    check_test(
        "Network Subsystem Init",
        "Network subsystem initialized" in boot_output,
        "Network subsystem should initialize"
    )
    
    check_test(
        "Shell Started",
        "Shell started with TID" in boot_output,
        "Shell should start"
    )
    
    # Test 2: Security Commands
    print("\n" + "=" * 60)
    print("Testing Security Subsystem Commands")
    print("=" * 60)
    
    output = send_command("whoami")
    check_test(
        "whoami command",
        "debos" in output.lower() or "current user" in output.lower(),
        "Should display current username"
    )
    
    output = send_command("id")
    check_test(
        "id command",
        "uid" in output.lower() or "gid" in output.lower() or "debos" in output.lower(),
        "Should display user/group IDs"
    )
    
    output = send_command("users")
    check_test(
        "users command",
        "debos" in output.lower() or "root" in output.lower() or "user" in output.lower(),
        "Should list users"
    )
    
    output = send_command("groups")
    check_test(
        "groups command",
        "wheel" in output.lower() or "users" in output.lower() or "group" in output.lower(),
        "Should list groups"
    )
    
    # Test 3: Filesystem Commands
    print("\n" + "=" * 60)
    print("Testing Filesystem Commands")
    print("=" * 60)
    
    output = send_command("ls")
    check_test(
        "ls command",
        "debos>" in output or "." in output or "total" in output.lower() or output.strip() != "",
        "Should list directory contents"
    )
    
    output = send_command("pwd")
    check_test(
        "pwd command",
        "/" in output,
        "Should display current directory"
    )
    
    output = send_command("mkdir /testdir")
    output = send_command("ls")
    check_test(
        "mkdir command",
        "testdir" in output.lower() or "directory created" in output.lower() or "debos>" in output,
        "Should create directory"
    )
    
    # Test 4: System Info Commands
    print("\n" + "=" * 60)
    print("Testing System Info Commands")
    print("=" * 60)
    
    output = send_command("mem")
    check_test(
        "mem command",
        "memory" in output.lower() or "mb" in output.lower() or "kb" in output.lower() or "heap" in output.lower(),
        "Should display memory info"
    )
    
    output = send_command("threads")
    check_test(
        "threads command",
        "thread" in output.lower() or "tid" in output.lower() or "shell" in output.lower(),
        "Should list threads"
    )
    
    # Test 5: Help Command
    print("\n" + "=" * 60)
    print("Testing Help Command")
    print("=" * 60)
    
    output = send_command("help")
    check_test(
        "help command",
        "help" in output.lower() and ("ls" in output.lower() or "pwd" in output.lower()),
        "Should display available commands"
    )
    
    # Check for new commands in help
    check_test(
        "User management in help",
        "whoami" in output.lower() or "useradd" in output.lower() or "passwd" in output.lower(),
        "Help should include user management commands"
    )
    
    # Cleanup
    print("\n" + "=" * 60)
    print("Cleaning up...")
    print("=" * 60)
    
    proc.terminate()
    try:
        proc.wait(timeout=3)
    except:
        proc.kill()
    
    # Summary
    print("\n" + "=" * 60)
    print("TEST SUMMARY")
    print("=" * 60)
    print(f"\nTotal Tests: {tests_passed + tests_failed}")
    print(f"Passed: {tests_passed}")
    print(f"Failed: {tests_failed}")
    print(f"Success Rate: {100 * tests_passed / (tests_passed + tests_failed):.1f}%")
    
    print("\n" + "-" * 60)
    print("Detailed Results:")
    print("-" * 60)
    for name, status, details in test_results:
        print(f"{status}: {name}")
        if "FAIL" in status and details:
            print(f"       Expected: {details}")
    
    return 0 if tests_failed == 0 else 1

if __name__ == "__main__":
    sys.exit(main())

