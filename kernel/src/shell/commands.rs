//! Shell Commands
//!
//! Built-in commands for the DebOS shell.

use crate::{println, print};
use crate::scheduler;
use crate::memory;
use crate::fs::{self, InodeType};
use alloc::string::String;

// ============================================================================
// Help Command
// ============================================================================

/// Display help information
pub fn help(_args: &[&str]) {
    println!("DebOS Shell Commands:");
    println!("=====================");
    println!();
    println!("System Commands:");
    println!("  help, ?        - Show this help message");
    println!("  info, sysinfo  - Display system information");
    println!("  mem, memory    - Show memory statistics");
    println!("  threads, ps    - List running threads");
    println!("  uptime         - Show system uptime");
    println!("  clear, cls     - Clear the screen");
    println!("  exit, quit     - Exit the shell");
    println!("  reboot         - Reboot the system");
    println!("  shutdown       - Power off the system");
    println!();
    println!("File Commands (RamFS):");
    println!("  pwd            - Print working directory");
    println!("  ls [path]      - List directory contents");
    println!("  cd <path>      - Change directory");
    println!("  mkdir <path>   - Create directory");
    println!("  rmdir <path>   - Remove empty directory");
    println!("  touch <file>   - Create empty file");
    println!("  cat <file>     - Display file contents");
    println!("  head [-n N] f  - Show first N lines (default 10)");
    println!("  tail [-n N] f  - Show last N lines (default 10)");
    println!("  rm <file>      - Remove file");
    println!("  write <f> <t>  - Write text to file");
    println!("  stat <path>    - Show file/dir info");
    println!();
    println!("Text Processing:");
    println!("  grep <pat> <f> - Search for pattern in file");
    println!("  edit <file>    - Edit file (vim-like)");
    println!();
    println!("Block Device (FAT32):");
    println!("  disk           - Show block device info");
    println!("  blkread <sec>  - Read a sector from disk");
    println!("  mount          - Mount FAT32 filesystem");
    println!("  fatls [path]   - List FAT32 directory");
    println!("  fatcat <file>  - Read FAT32 file");
    println!("  fatwrite <f> t - Write text to FAT32 file");
    println!("  fatrm <file>   - Delete FAT32 file");
    println!();
    println!("User & Security:");
    println!("  whoami         - Show current user");
    println!("  id             - Show user/group info");
    println!("  users          - List all users");
    println!("  groups         - List all groups");
    println!("  useradd <name> - Create new user");
    println!("  userdel <name> - Delete user");
    println!("  passwd [user]  - Change password");
    println!("  su [user]      - Switch user");
    println!("  sudo <cmd>     - Run as admin");
    println!("  login          - Login as user");
    println!();
    println!("Networking:");
    println!("  ifconfig       - Show network interfaces");
    println!("  ping <host>    - Ping a host (ICMP)");
    println!("  arp            - Show ARP cache");
    println!("  netstat        - Show network stats");
    println!();
    println!("Device Info:");
    println!("  devices        - List all devices");
    println!("  lspci          - List PCI devices");
    println!("  lsusb          - List USB devices");
    println!();
    println!("Other:");
    println!("  echo <text>    - Echo text to console");
    println!();
}

// ============================================================================
// System Commands
// ============================================================================

/// Display system information
pub fn sysinfo(_args: &[&str]) {
    println!("DebOS System Information");
    println!("========================");
    println!();
    
    #[cfg(target_arch = "x86_64")]
    println!("  Architecture:  x86_64");
    
    #[cfg(target_arch = "aarch64")]
    println!("  Architecture:  AArch64 (ARM64)");
    
    println!("  Kernel:        DeK (DebOS Nano-Kernel) v0.1.0");
    println!("  Type:          Microkernel");
    
    // Memory info
    let (heap_used, heap_free) = memory::heap::stats();
    println!("  Heap Used:     {} KB", heap_used / 1024);
    println!("  Heap Free:     {} KB", heap_free / 1024);
    
    // Uptime
    let ticks = scheduler::ticks();
    let seconds = ticks / 100;
    println!("  Uptime:        {} seconds ({} ticks)", seconds, ticks);
    
    println!();
}

/// Display memory statistics
pub fn memory(_args: &[&str]) {
    println!("Memory Statistics");
    println!("=================");
    println!();
    
    let (heap_used, heap_free) = memory::heap::stats();
    let heap_total = heap_used + heap_free;
    
    let used_pct = if heap_total > 0 { (heap_used * 100) / heap_total } else { 0 };
    let free_pct = if heap_total > 0 { (heap_free * 100) / heap_total } else { 0 };
    
    println!("  Heap:");
    println!("    Total:  {} KB", heap_total / 1024);
    println!("    Used:   {} KB ({}%)", heap_used / 1024, used_pct);
    println!("    Free:   {} KB ({}%)", heap_free / 1024, free_pct);
    
    let (phys_total, phys_free) = memory::buddy::stats();
    if phys_total > 0 {
        println!();
        println!("  Physical Memory (Buddy Allocator):");
        println!("    Total:  {} MB", phys_total / 1024 / 1024);
        println!("    Free:   {} MB", phys_free / 1024 / 1024);
    }
    
    println!();
}

/// List running threads
pub fn threads(_args: &[&str]) {
    println!("Running Threads");
    println!("===============");
    println!();
    println!("  TID   State      Priority   Name");
    println!("  ---   -----      --------   ----");
    
    if let Some(tid) = scheduler::current_tid() {
        println!("  {}     Running    128        shell", tid);
    }
    
    println!();
    println!("  (Full thread listing not yet implemented)");
    println!();
}

/// Display system uptime
pub fn uptime(_args: &[&str]) {
    let ticks = scheduler::ticks();
    let total_seconds = ticks / 100;
    
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    
    println!("System uptime: {}h {}m {}s ({} ticks)", hours, minutes, seconds, ticks);
}

/// Echo text to console
pub fn echo(args: &[&str]) {
    if args.is_empty() {
        println!();
    } else {
        println!("{}", args.join(" "));
    }
}

/// Clear the screen
pub fn clear(_args: &[&str]) {
    print!("\x1B[2J\x1B[H");
}

/// Reboot the system
pub fn reboot(_args: &[&str]) {
    println!("Rebooting system...");

    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            use x86_64::instructions::port::Port;
            let mut port = Port::<u8>::new(0x64);
            port.write(0xFE);
        }
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        // PSCI SYSTEM_RESET via HVC on QEMU virt machine
        core::arch::asm!(
            "ldr x0, =0x84000009",
            "hvc #0",
            options(noreturn)
        );
    }
}

pub fn shutdown(_args: &[&str]) {
    println!("Shutting down...");

    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            // QEMU shutdown via ACPI (isa-debug-exit or ACPI PM1a)
            use x86_64::instructions::port::Port;
            let mut port = Port::<u16>::new(0x604);
            port.write(0x2000);
        }
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        // PSCI SYSTEM_OFF via HVC on QEMU virt machine
        core::arch::asm!(
            "ldr x0, =0x84000008",
            "hvc #0",
            options(noreturn)
        );
    }
}

// ============================================================================
// Block Device Commands
// ============================================================================

/// Show block device information
pub fn disk(_args: &[&str]) {
    println!("Block Device Information");
    println!("========================");
    println!();
    
    // Get VirtIO device count
    let virtio_count = crate::drivers::virtio::device_count();
    println!("  VirtIO devices discovered: {}", virtio_count);
    println!();
    
    // Get block device info
    if let Some((capacity, block_size, read_only)) = crate::drivers::block::get_device_info() {
        println!("  Block Device:");
        println!("    Capacity:   {} sectors ({} MB)",
            capacity, 
            capacity * block_size as u64 / 1024 / 1024);
        println!("    Block size: {} bytes", block_size);
        println!("    Read-only:  {}", if read_only { "Yes" } else { "No" });
    } else {
        println!("  No block device available");
        println!();
        println!("  Tip: Start QEMU with a disk:");
        println!("    -drive file=disk.img,format=raw,if=none,id=hd0");
        println!("    -device virtio-blk-device,drive=hd0");
    }
    println!();
}

/// Read a sector from the block device
pub fn blkread(args: &[&str]) {
    if args.is_empty() {
        println!("Usage: blkread <sector>");
        return;
    }
    
    let sector: u64 = match args[0].parse() {
        Ok(s) => s,
        Err(_) => {
            println!("blkread: invalid sector number");
            return;
        }
    };
    
    let mut buf = [0u8; 512];
    match crate::drivers::virtio::block::read_sector(sector, &mut buf) {
        Ok(_) => {
            println!("Sector {} contents:", sector);
            println!();
            
            // Hexdump the first 256 bytes
            for row in 0..16 {
                let offset = row * 16;
                print!("  {:04x}:  ", offset);
                
                // Hex
                for col in 0..16 {
                    print!("{:02x} ", buf[offset + col]);
                    if col == 7 {
                        print!(" ");
                    }
                }
                
                // ASCII
                print!(" |");
                for col in 0..16 {
                    let ch = buf[offset + col];
                    if ch >= 0x20 && ch < 0x7f {
                        print!("{}", ch as char);
                    } else {
                        print!(".");
                    }
                }
                println!("|");
            }
            println!();
        }
        Err(e) => {
            println!("blkread: failed to read sector {}: {:?}", sector, e);
        }
    }
}

/// Mount FAT32 filesystem from block device
pub fn mount(_args: &[&str]) {
    use crate::fs::fat32;
    
    if fat32::is_mounted() {
        println!("FAT32 filesystem is already mounted");
        return;
    }
    
    if crate::drivers::block::get_device_info().is_none() {
        println!("mount: no block device available");
        return;
    }
    
    match fat32::mount() {
        Ok(_) => println!("FAT32 filesystem mounted successfully"),
        Err(e) => println!("mount: failed to mount FAT32: {:?}", e),
    }
}

/// List FAT32 directory
pub fn fatls(args: &[&str]) {
    use crate::fs::fat32;
    
    if !fat32::is_mounted() {
        println!("fatls: FAT32 not mounted (use 'mount' first)");
        return;
    }
    
    let path = args.first().copied().unwrap_or("/");
    
    match fat32::ls(path) {
        Ok(entries) => {
            if entries.is_empty() {
                println!("(empty directory)");
                return;
            }
            
            println!("Directory of {}", path);
            println!();
            
            for (name, is_dir, size) in entries {
                if is_dir {
                    println!("  <DIR>        {}", name);
                } else {
                    println!("  {:>10}  {}", size, name);
                }
            }
            println!();
        }
        Err(e) => {
            println!("fatls: {:?}", e);
        }
    }
}

/// Read FAT32 file
pub fn fatcat(args: &[&str]) {
    use crate::fs::fat32;
    
    if !fat32::is_mounted() {
        println!("fatcat: FAT32 not mounted (use 'mount' first)");
        return;
    }
    
    if args.is_empty() {
        println!("Usage: fatcat <file>");
        return;
    }
    
    let path = args[0];
    
    match fat32::read_file(path) {
        Ok(data) => {
            // Try to print as text
            if let Ok(text) = core::str::from_utf8(&data) {
                print!("{}", text);
                if !text.ends_with('\n') {
                    println!();
                }
            } else {
                // Binary file, show hexdump
                println!("(Binary file, {} bytes)", data.len());
                let to_show = data.len().min(256);
                for row in 0..(to_show + 15) / 16 {
                    let offset = row * 16;
                    print!("  {:04x}:  ", offset);
                    for col in 0..16 {
                        if offset + col < to_show {
                            print!("{:02x} ", data[offset + col]);
                        } else {
                            print!("   ");
                        }
                    }
                    println!();
                }
            }
        }
        Err(e) => {
            println!("fatcat: {:?}", e);
        }
    }
}

/// Write to FAT32 file
pub fn fatwrite(args: &[&str]) {
    use crate::fs::fat32;
    
    if !fat32::is_mounted() {
        println!("fatwrite: FAT32 not mounted (use 'mount' first)");
        return;
    }
    
    if args.len() < 2 {
        println!("Usage: fatwrite <file> <text...>");
        return;
    }
    
    let path = args[0];
    let text = args[1..].join(" ");
    let data = text.as_bytes();
    
    match fat32::write_file(path, data) {
        Ok(_) => println!("Wrote {} bytes to {}", data.len(), path),
        Err(e) => println!("fatwrite: {:?}", e),
    }
}

/// Delete FAT32 file
pub fn fatrm(args: &[&str]) {
    use crate::fs::fat32;
    
    if !fat32::is_mounted() {
        println!("fatrm: FAT32 not mounted (use 'mount' first)");
        return;
    }
    
    if args.is_empty() {
        println!("Usage: fatrm <file>");
        return;
    }
    
    let path = args[0];
    
    match fat32::delete_file(path) {
        Ok(_) => println!("Deleted {}", path),
        Err(e) => println!("fatrm: {:?}", e),
    }
}

// ============================================================================
// Text Processing Commands
// ============================================================================

/// Show first N lines of a file (head)
pub fn head(args: &[&str]) {
    let mut lines = 10usize;
    let mut file_idx = 0;
    
    // Parse arguments
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-n" && i + 1 < args.len() {
            lines = args[i + 1].parse().unwrap_or(10);
            i += 2;
        } else {
            file_idx = i;
            break;
        }
        i += 1;
    }
    
    if file_idx >= args.len() {
        println!("Usage: head [-n lines] <file>");
        return;
    }
    
    let path = args[file_idx];
    
    match fs::read_to_string(path) {
        Ok(content) => {
            for (i, line) in content.lines().enumerate() {
                if i >= lines {
                    break;
                }
                println!("{}", line);
            }
        }
        Err(e) => println!("head: {}: {}", path, e),
    }
}

/// Show last N lines of a file (tail)
pub fn tail(args: &[&str]) {
    let mut lines = 10usize;
    let mut file_idx = 0;
    
    // Parse arguments
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-n" && i + 1 < args.len() {
            lines = args[i + 1].parse().unwrap_or(10);
            i += 2;
        } else {
            file_idx = i;
            break;
        }
        i += 1;
    }
    
    if file_idx >= args.len() {
        println!("Usage: tail [-n lines] <file>");
        return;
    }
    
    let path = args[file_idx];
    
    match fs::read_to_string(path) {
        Ok(content) => {
            let all_lines: alloc::vec::Vec<&str> = content.lines().collect();
            let start = if all_lines.len() > lines { all_lines.len() - lines } else { 0 };
            
            for line in &all_lines[start..] {
                println!("{}", line);
            }
        }
        Err(e) => println!("tail: {}: {}", path, e),
    }
}

/// Search for pattern in file (grep)
pub fn grep(args: &[&str]) {
    if args.len() < 2 {
        println!("Usage: grep <pattern> <file>");
        return;
    }
    
    let pattern = args[0];
    let path = args[1];
    
    match fs::read_to_string(path) {
        Ok(content) => {
            let mut found = false;
            for (line_num, line) in content.lines().enumerate() {
                if line.contains(pattern) {
                    println!("{}:{}: {}", path, line_num + 1, line);
                    found = true;
                }
            }
            if !found {
                println!("(no matches)");
            }
        }
        Err(e) => println!("grep: {}: {}", path, e),
    }
}

// ============================================================================
// Text Editor (vim-like)
// ============================================================================

/// Simple line-based text editor
pub fn edit(args: &[&str]) {
    if args.is_empty() {
        println!("Usage: edit <file>");
        return;
    }
    
    let path = args[0];
    
    // Load existing content or start fresh
    let mut lines: alloc::vec::Vec<alloc::string::String> = match fs::read_to_string(path) {
        Ok(content) => content.lines().map(|s| alloc::string::String::from(s)).collect(),
        Err(_) => alloc::vec::Vec::new(),
    };
    
    if lines.is_empty() {
        lines.push(alloc::string::String::new());
    }
    
    let mut current_line = 0usize;
    let mut modified = false;
    
    println!("=== DebOS Editor ===");
    println!("Commands: i (insert), d (delete line), p (print), g N (goto line)");
    println!("          w (write), q (quit), wq (write & quit), h (help)");
    println!();
    
    // Show initial content
    print_editor_lines(&lines, current_line);
    
    loop {
        print!(":{} > ", current_line + 1);
        
        if let Some(cmd) = read_editor_line() {
            let cmd = cmd.trim();
            
            if cmd.is_empty() {
                continue;
            }
            
            let parts: alloc::vec::Vec<&str> = cmd.splitn(2, ' ').collect();
            let command = parts[0];
            let arg = parts.get(1).copied().unwrap_or("");
            
            match command {
                "h" | "help" => {
                    println!("Editor Commands:");
                    println!("  i <text>   - Insert text at current line");
                    println!("  a <text>   - Append text after current line");
                    println!("  d          - Delete current line");
                    println!("  r <text>   - Replace current line");
                    println!("  p          - Print all lines");
                    println!("  g <n>      - Go to line n");
                    println!("  n          - Next line");
                    println!("  N          - Previous line");
                    println!("  w          - Write file");
                    println!("  q          - Quit (warns if unsaved)");
                    println!("  wq         - Write and quit");
                }
                "i" => {
                    lines.insert(current_line, alloc::string::String::from(arg));
                    modified = true;
                    println!("Inserted at line {}", current_line + 1);
                }
                "a" => {
                    current_line += 1;
                    if current_line > lines.len() {
                        current_line = lines.len();
                    }
                    lines.insert(current_line, alloc::string::String::from(arg));
                    modified = true;
                    println!("Appended at line {}", current_line + 1);
                }
                "d" => {
                    if lines.len() > 1 {
                        lines.remove(current_line);
                        if current_line >= lines.len() {
                            current_line = lines.len() - 1;
                        }
                        modified = true;
                        println!("Deleted line");
                    } else {
                        lines[0] = alloc::string::String::new();
                        modified = true;
                        println!("Cleared line");
                    }
                }
                "r" => {
                    lines[current_line] = alloc::string::String::from(arg);
                    modified = true;
                    println!("Replaced line {}", current_line + 1);
                }
                "p" => {
                    print_editor_lines(&lines, current_line);
                }
                "g" => {
                    if let Ok(n) = arg.parse::<usize>() {
                        if n > 0 && n <= lines.len() {
                            current_line = n - 1;
                            println!("Line {}: {}", current_line + 1, lines[current_line]);
                        } else {
                            println!("Invalid line number");
                        }
                    }
                }
                "n" => {
                    if current_line + 1 < lines.len() {
                        current_line += 1;
                        println!("Line {}: {}", current_line + 1, lines[current_line]);
                    }
                }
                "N" => {
                    if current_line > 0 {
                        current_line -= 1;
                        println!("Line {}: {}", current_line + 1, lines[current_line]);
                    }
                }
                "w" => {
                    let content = lines.join("\n");
                    match fs::write_string(path, &content) {
                        Ok(_) => {
                            modified = false;
                            println!("Wrote {} lines to {}", lines.len(), path);
                        }
                        Err(e) => println!("Error writing: {}", e),
                    }
                }
                "q" => {
                    if modified {
                        println!("Unsaved changes! Use 'wq' to save and quit, or 'q!' to force quit");
                    } else {
                        break;
                    }
                }
                "q!" => {
                    break;
                }
                "wq" => {
                    let content = lines.join("\n");
                    match fs::write_string(path, &content) {
                        Ok(_) => {
                            println!("Wrote {} lines to {}", lines.len(), path);
                            break;
                        }
                        Err(e) => println!("Error writing: {}", e),
                    }
                }
                _ => {
                    println!("Unknown command. Type 'h' for help.");
                }
            }
        }
    }
    
    println!("Editor closed.");
}

/// Print editor lines with current line indicator
fn print_editor_lines(lines: &[alloc::string::String], current: usize) {
    for (i, line) in lines.iter().enumerate() {
        let marker = if i == current { ">" } else { " " };
        println!("{} {:3}: {}", marker, i + 1, line);
    }
}

/// Read a line in editor mode
fn read_editor_line() -> Option<alloc::string::String> {
    use crate::shell::input;
    
    let mut buffer = alloc::string::String::new();
    
    loop {
        if let Some(c) = input::read_char() {
            match c {
                b'\r' | b'\n' => {
                    println!();
                    return Some(buffer);
                }
                0x7F | 0x08 => {
                    if !buffer.is_empty() {
                        buffer.pop();
                        print!("\x08 \x08");
                    }
                }
                0x03 => {
                    println!("^C");
                    return None;
                }
                c if c >= 0x20 && c < 0x7F => {
                    buffer.push(c as char);
                    print!("{}", c as char);
                }
                _ => {}
            }
        }
        
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
    }
}

// ============================================================================
// Filesystem Commands
// ============================================================================

/// Print working directory
pub fn pwd(_args: &[&str]) {
    match fs::getcwd() {
        Ok(path) => println!("{}", path),
        Err(e) => println!("pwd: {}", e),
    }
}

/// List directory contents
pub fn ls(args: &[&str]) {
    let path = args.first().copied().unwrap_or(".");
    
    match fs::readdir(path) {
        Ok(entries) => {
            if entries.is_empty() {
                // Empty directory, just return silently
                return;
            }
            
            for entry in entries {
                let type_char = match entry.inode_type {
                    InodeType::Directory => 'd',
                    InodeType::File => '-',
                    InodeType::Symlink => 'l',
                };
                
                // Format: type size name
                if entry.inode_type == InodeType::Directory {
                    println!("{}  {:>8}  {}/", type_char, entry.size, entry.name);
                } else {
                    println!("{}  {:>8}  {}", type_char, entry.size, entry.name);
                }
            }
        }
        Err(e) => println!("ls: {}: {}", path, e),
    }
}

/// Change directory
pub fn cd(args: &[&str]) {
    let path = args.first().copied().unwrap_or("/");
    
    if let Err(e) = fs::chdir(path) {
        println!("cd: {}: {}", path, e);
    }
}

/// Create directory
pub fn mkdir(args: &[&str]) {
    if args.is_empty() {
        println!("mkdir: missing operand");
        println!("Usage: mkdir <directory>...");
        return;
    }
    
    for path in args {
        // Execute mkdir and ensure it completes
        let result = fs::mkdir(path);
        
        // Explicitly handle the result to ensure function completes
        match result {
            Ok(()) => {
                // Success - directory created (no output needed, like Unix mkdir)
                // Debug: Uncomment to verify mkdir completes
                // println!("[DEBUG] mkdir '{}' completed", path);
            }
            Err(e) => {
                println!("mkdir: cannot create directory '{}': {}", path, e);
            }
        }
    }
    
    // Function returns here - if we reach this point, mkdir completed successfully
}

/// Remove empty directory
pub fn rmdir(args: &[&str]) {
    if args.is_empty() {
        println!("rmdir: missing operand");
        println!("Usage: rmdir <directory>...");
        return;
    }
    
    for path in args {
        if let Err(e) = fs::rmdir(path) {
            println!("rmdir: failed to remove '{}': {}", path, e);
        }
    }
}

/// Create empty file or update timestamp
pub fn touch(args: &[&str]) {
    if args.is_empty() {
        println!("touch: missing operand");
        println!("Usage: touch <file>...");
        return;
    }
    
    for path in args {
        if let Err(e) = fs::touch(path) {
            println!("touch: cannot touch '{}': {}", path, e);
        }
    }
}

/// Display file contents
pub fn cat(args: &[&str]) {
    if args.is_empty() {
        println!("cat: missing operand");
        println!("Usage: cat <file>...");
        return;
    }
    
    for path in args {
        match fs::read_to_string(path) {
            Ok(contents) => {
                print!("{}", contents);
                // Add newline if file doesn't end with one
                if !contents.ends_with('\n') {
                    println!();
                }
            }
            Err(e) => println!("cat: {}: {}", path, e),
        }
    }
}

/// Remove file
pub fn rm(args: &[&str]) {
    if args.is_empty() {
        println!("rm: missing operand");
        println!("Usage: rm <file>...");
        return;
    }
    
    for path in args {
        if let Err(e) = fs::unlink(path) {
            println!("rm: cannot remove '{}': {}", path, e);
        }
    }
}

/// Write text to file
pub fn write_file(args: &[&str]) {
    if args.len() < 2 {
        println!("write: missing operand");
        println!("Usage: write <filename> <content>...");
        return;
    }
    
    let path = args[0];
    let content = args[1..].join(" ");
    
    // Add newline at the end
    let content_with_newline = alloc::format!("{}\n", content);
    
    if let Err(e) = fs::write_string(path, &content_with_newline) {
        println!("write: {}: {}", path, e);
    }
}

/// Show file/directory information
pub fn stat_cmd(args: &[&str]) {
    if args.is_empty() {
        println!("stat: missing operand");
        println!("Usage: stat <file>...");
        return;
    }
    
    for path in args {
        match fs::stat(path) {
            Ok(st) => {
                let type_str = match st.inode_type {
                    InodeType::File => "regular file",
                    InodeType::Directory => "directory",
                    InodeType::Symlink => "symbolic link",
                };
                
                println!("  File: {}", path);
                println!("  Size: {}  Type: {}", st.size, type_str);
                println!("  Inode: {}  Permissions: {:o}", st.inode, st.permissions);
            }
            Err(e) => println!("stat: cannot stat '{}': {}", path, e),
        }
    }
}

/// Tree command - display directory structure
pub fn tree(args: &[&str]) {
    let path = args.first().copied().unwrap_or(".");
    
    println!("{}", path);
    if let Err(e) = print_tree(path, "", true) {
        println!("tree: {}: {}", path, e);
    }
}

/// Helper function for tree command
fn print_tree(path: &str, prefix: &str, is_last: bool) -> Result<(), fs::FsError> {
    let entries = fs::readdir(path)?;
    let count = entries.len();
    
    for (i, entry) in entries.iter().enumerate() {
        let is_last_entry = i == count - 1;
        let connector = if is_last_entry { "└── " } else { "├── " };
        
        if entry.inode_type == InodeType::Directory {
            println!("{}{}{}/", prefix, connector, entry.name);
            
            let new_prefix = alloc::format!(
                "{}{}",
                prefix,
                if is_last_entry { "    " } else { "│   " }
            );
            
            let subpath = if path == "/" || path == "." {
                alloc::format!("/{}", entry.name)
            } else {
                alloc::format!("{}/{}", path, entry.name)
            };
            
            let _ = print_tree(&subpath, &new_prefix, is_last_entry);
        } else {
            println!("{}{}{}", prefix, connector, entry.name);
        }
    }
    
    Ok(())
}

// ============================================================================
// User & Security Commands
// ============================================================================

/// Show current user
pub fn whoami(_args: &[&str]) {
    use crate::shell::sdk;
    
    // Use SDK for safe credential access (no deadlock)
    println!("{}", sdk::current_username());
}

/// Show user and group info
pub fn id(args: &[&str]) {
    use crate::shell::sdk;
    use crate::security::{database, identity::GroupId};
    
    let username = if args.is_empty() {
        sdk::current_username()
    } else {
        alloc::string::String::from(args[0])
    };
    
    if let Some(user) = database::get_user_by_name(&username) {
        print!("uid={}({}) ", user.uid, user.username);
        
        if let Some(group) = database::get_group_by_gid(user.gid) {
            print!("gid={}({}) ", user.gid, group.name);
        } else {
            print!("gid={} ", user.gid);
        }
        
        print!("groups=");
        let mut first = true;
        for gid in &user.groups {
            if !first {
                print!(",");
            }
            if let Some(group) = database::get_group_by_gid(*gid) {
                print!("{}({})", gid, group.name);
            } else {
                print!("{}", gid);
            }
            first = false;
        }
        println!();
        
        if user.is_admin {
            println!("  (administrator)");
        }
    } else {
        println!("id: '{}': no such user", username);
    }
}

/// List all users
pub fn users_list(_args: &[&str]) {
    use crate::security::database;
    
    println!("Users:");
    println!("{:<12} {:>6} {:>6} {:<8} {}", "USERNAME", "UID", "GID", "STATUS", "HOME");
    println!("{}", "-".repeat(60));
    
    for user in database::list_users() {
        let status = match user.status {
            crate::security::identity::AccountStatus::Active => "active",
            crate::security::identity::AccountStatus::Locked => "locked",
            crate::security::identity::AccountStatus::Disabled => "disabled",
            crate::security::identity::AccountStatus::Expired => "expired",
        };
        
        let admin_mark = if user.is_admin { "*" } else { " " };
        
        println!("{:<12} {:>6} {:>6} {:<8}{} {}", 
            user.username, user.uid, user.gid, status, admin_mark, user.home_dir);
    }
    println!();
    println!("* = administrator (wheel group member)");
}

/// List all groups
pub fn groups_list(_args: &[&str]) {
    use crate::security::database;
    
    println!("Groups:");
    println!("{:<16} {:>6} {}", "NAME", "GID", "MEMBERS");
    println!("{}", "-".repeat(50));
    
    for group in database::list_groups() {
        let members: alloc::vec::Vec<_> = group.members.iter()
            .filter_map(|uid| database::get_username(*uid))
            .collect();
        
        println!("{:<16} {:>6} {}", group.name, group.gid, members.join(", "));
    }
}

/// Create a new user
pub fn useradd(args: &[&str]) {
    use crate::security::{self, database, auth};
    
    // Check if running as admin
    if !security::is_root() && !security::has_capability(security::capability::Capability::DebosUserAdmin) {
        // Check if current user is admin
        let uid = crate::shell::sdk::current_uid();
        if let Some(user) = database::get_user_by_uid(uid) {
            if !user.is_admin {
                println!("useradd: permission denied (requires admin)");
                return;
            }
        } else {
            println!("useradd: permission denied");
            return;
        }
    }
    
    if args.is_empty() {
        println!("Usage: useradd <username> [-a] [-p <password>]");
        println!("  -a          Make user an administrator");
        println!("  -p <pass>   Set initial password");
        return;
    }
    
    let username = args[0];
    let mut is_admin = false;
    let mut password: Option<&str> = None;
    
    let mut i = 1;
    while i < args.len() {
        match args[i] {
            "-a" => is_admin = true,
            "-p" if i + 1 < args.len() => {
                password = Some(args[i + 1]);
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    
    match database::create_user(username, "", None, None, is_admin, password) {
        Ok(user) => {
            println!("User '{}' created (uid={})", username, user.uid);
            if is_admin {
                println!("  User is an administrator");
            }
            if password.is_none() {
                println!("  No password set (passwordless login)");
            }
        }
        Err(e) => {
            println!("useradd: {}", e);
        }
    }
}

/// Delete a user
pub fn userdel(args: &[&str]) {
    use crate::security::{self, database};
    
    // Check permissions
    if !security::is_root() {
        let uid = crate::shell::sdk::current_uid();
        if let Some(user) = database::get_user_by_uid(uid) {
            if !user.is_admin {
                println!("userdel: permission denied (requires admin)");
                return;
            }
        } else {
            println!("userdel: permission denied");
            return;
        }
    }
    
    if args.is_empty() {
        println!("Usage: userdel <username>");
        return;
    }
    
    let username = args[0];
    
    match database::delete_user(username) {
        Ok(_) => println!("User '{}' deleted", username),
        Err(e) => println!("userdel: {}", e),
    }
}

/// Change password
pub fn passwd(args: &[&str]) {
    use crate::security::{self, database, auth};
    use crate::shell::input;
    
    let target_user = if args.is_empty() {
        // Change own password
        let uid = crate::shell::sdk::current_uid();
        database::get_username(uid).unwrap_or_else(|| alloc::format!("uid={}", uid))
    } else {
        // Changing another user's password requires admin
        if !security::is_root() {
            let uid = crate::shell::sdk::current_uid();
            if let Some(user) = database::get_user_by_uid(uid) {
                if !user.is_admin {
                    println!("passwd: permission denied (requires admin to change other's password)");
                    return;
                }
            }
        }
        alloc::string::String::from(args[0])
    };
    
    // Check if user exists
    if database::get_user_by_name(&target_user).is_none() {
        println!("passwd: user '{}' does not exist", target_user);
        return;
    }
    
    println!("Changing password for {}", target_user);
    print!("New password (empty for no password): ");
    
    // Read password (we don't have proper no-echo, so just read)
    if let Some(password) = read_password_line() {
        match auth::set_password(&target_user, &password) {
            Ok(_) => {
                if password.is_empty() {
                    println!("Password removed (passwordless login enabled)");
                } else {
                    println!("Password updated successfully");
                }
            }
            Err(e) => println!("passwd: {}", e),
        }
    }
}

/// Read a password line (simple version)
fn read_password_line() -> Option<alloc::string::String> {
    use crate::shell::input;
    
    let mut buffer = alloc::string::String::new();
    
    loop {
        if let Some(c) = input::read_char() {
            match c {
                b'\r' | b'\n' => {
                    println!();
                    return Some(buffer);
                }
                0x7F | 0x08 => {
                    if !buffer.is_empty() {
                        buffer.pop();
                    }
                }
                0x03 => {
                    println!("^C");
                    return None;
                }
                c if c >= 0x20 && c < 0x7F => {
                    buffer.push(c as char);
                    // Don't echo password
                }
                _ => {}
            }
        }
        
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
    }
}

/// Switch user (su)
pub fn su(args: &[&str]) {
    use crate::security::{self, database, auth};
    
    let target_user = if args.is_empty() {
        alloc::string::String::from("root")
    } else {
        alloc::string::String::from(args[0])
    };
    
    // Check if user exists
    let user = match database::get_user_by_name(&target_user) {
        Some(u) => u,
        None => {
            println!("su: user '{}' does not exist", target_user);
            return;
        }
    };
    
    // Check account status
    if user.status != crate::security::identity::AccountStatus::Active {
        println!("su: account is not active");
        return;
    }
    
    // For switching to root, check if current user is admin
    if target_user == "root" {
        let current_uid = crate::shell::sdk::current_uid();
        if let Some(current_user) = database::get_user_by_uid(current_uid) {
            if !current_user.is_admin {
                println!("su: permission denied (requires admin privileges)");
                return;
            }
        }
    }
    
    // Authenticate if password is required
    if !auth::is_passwordless(&target_user) {
        print!("Password: ");
        if let Some(password) = read_password_line() {
            match auth::authenticate(&target_user, &password) {
                auth::AuthResult::Success(_) | auth::AuthResult::NoPasswordRequired(_) => {
                    // Continue
                }
                _ => {
                    println!("su: authentication failure");
                    return;
                }
            }
        } else {
            return;
        }
    }
    
    // Create new credentials
    let creds = auth::create_session(&user);
    
    // Set credentials for current thread
    if let Err(e) = crate::scheduler::set_credentials(creds.clone()) {
        println!("su: failed to switch user: {}", e);
        return;
    }
    
    // Update SDK cache to reflect new credentials
    crate::shell::sdk::update_credentials(&creds);
    
    println!("Switched to user: {}", target_user);
}

/// Run command as admin (sudo)
pub fn sudo(args: &[&str]) {
    use crate::security::{self, database, auth, policy};
    
    if args.is_empty() {
        println!("Usage: sudo <command> [args...]");
        return;
    }
    
    // Check if current user is admin
    let current_uid = crate::shell::sdk::current_uid();
    let current_user = match database::get_user_by_uid(current_uid) {
        Some(u) => u,
        None => {
            println!("sudo: unknown user");
            return;
        }
    };
    
    if !current_user.is_admin {
        println!("sudo: {} is not in the sudoers file. This incident will be reported.", 
            current_user.username);
        policy::audit_log(
            policy::AuditEventType::SudoFailed,
            &current_user.username,
            &alloc::format!("attempted: {}", args.join(" ")),
            false,
        );
        return;
    }
    
    // Authenticate if policy requires
    let sec_policy = policy::get_policy();
    if sec_policy.sudo_requires_password && !auth::is_passwordless(&current_user.username) {
        print!("[sudo] password for {}: ", current_user.username);
        if let Some(password) = read_password_line() {
            match auth::authenticate(&current_user.username, &password) {
                auth::AuthResult::Success(_) | auth::AuthResult::NoPasswordRequired(_) => {}
                _ => {
                    println!("sudo: authentication failure");
                    policy::audit_log(
                        policy::AuditEventType::SudoFailed,
                        &current_user.username,
                        "authentication failure",
                        false,
                    );
                    return;
                }
            }
        } else {
            return;
        }
    }
    
    // Temporarily elevate to root
    let old_creds = crate::scheduler::current_credentials();
    
    // Create root credentials
    let root_creds = crate::security::credentials::ProcessCredentials::root();
    
    let root_creds_clone = root_creds.clone();
    if let Err(e) = crate::scheduler::set_credentials(root_creds_clone.clone()) {
        println!("sudo: failed to elevate: {}", e);
        return;
    }
    // Update SDK cache for root
    crate::shell::sdk::update_credentials(&root_creds_clone);
    
    // Log the sudo action
    policy::audit_log(
        policy::AuditEventType::Sudo,
        &current_user.username,
        &args.join(" "),
        true,
    );
    
    // Execute the command (would normally call the command handler here)
    println!("[executing as root: {}]", args.join(" "));
    
    // Note: In a real implementation, we'd execute the command here
    // For now, just demonstrate the privilege elevation
    
    // Restore original credentials
    if let Some(creds) = old_creds.clone() {
        let _ = crate::scheduler::set_credentials(creds.clone());
        // Update SDK cache
        crate::shell::sdk::update_credentials(&creds);
    }
}

/// Login command
pub fn login(_args: &[&str]) {
    use crate::security::{self, database, auth};
    
    println!("DebOS Login");
    println!();
    
    print!("Username: ");
    if let Some(username) = read_input_line() {
        let username = username.trim();
        
        // Check if user exists
        if database::get_user_by_name(username).is_none() {
            println!("Login incorrect");
            return;
        }
        
        // Check if password required
        if auth::is_passwordless(username) {
            // Direct login
            match auth::authenticate(username, "") {
                auth::AuthResult::Success(user) | auth::AuthResult::NoPasswordRequired(user) => {
                    let creds = auth::create_session(&user);
                    if let Err(e) = crate::scheduler::set_credentials(creds.clone()) {
                        println!("Login failed: {}", e);
                        return;
                    }
                    // Update SDK cache
                    crate::shell::sdk::update_credentials(&creds);
                    println!("Welcome, {}!", username);
                }
                _ => {
                    println!("Login incorrect");
                }
            }
        } else {
            print!("Password: ");
            if let Some(password) = read_password_line() {
                match auth::authenticate(username, &password) {
                    auth::AuthResult::Success(user) => {
                        let creds = auth::create_session(&user);
                        if let Err(e) = crate::scheduler::set_credentials(creds) {
                            println!("Login failed: {}", e);
                            return;
                        }
                        println!("Welcome, {}!", username);
                    }
                    auth::AuthResult::InvalidPassword => {
                        println!("Login incorrect");
                    }
                    auth::AuthResult::AccountLocked => {
                        println!("Account is locked. Try again later.");
                    }
                    auth::AuthResult::AccountDisabled => {
                        println!("Account is disabled.");
                    }
                    _ => {
                        println!("Login failed");
                    }
                }
            }
        }
    }
}

/// Read input line
fn read_input_line() -> Option<alloc::string::String> {
    use crate::shell::input;
    
    let mut buffer = alloc::string::String::new();
    
    loop {
        if let Some(c) = input::read_char() {
            match c {
                b'\r' | b'\n' => {
                    println!();
                    return Some(buffer);
                }
                0x7F | 0x08 => {
                    if !buffer.is_empty() {
                        buffer.pop();
                        print!("\x08 \x08");
                    }
                }
                0x03 => {
                    println!("^C");
                    return None;
                }
                c if c >= 0x20 && c < 0x7F => {
                    buffer.push(c as char);
                    print!("{}", c as char);
                }
                _ => {}
            }
        }
        
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
    }
}

// ============================================================================
// Network Commands
// ============================================================================

/// Show network interface configuration
pub fn ifconfig(_args: &[&str]) {
    use crate::drivers::net;
    
    println!("Network Interfaces:");
    println!("===================");
    println!();
    
    for name in net::list_interfaces() {
        if let Some(iface) = net::get_interface(&name) {
            println!("{}: flags={}", name, if iface.up { "UP" } else { "DOWN" });
            println!("    mac:  {}", iface.mac);
            
            if let Some(ipv4) = iface.ipv4 {
                println!("    inet: {}", ipv4);
                if let Some(mask) = iface.netmask {
                    println!("    mask: {}", mask);
                }
                if let Some(gw) = iface.gateway {
                    println!("    gw:   {}", gw);
                }
            }
            
            println!("    mtu:  {}", iface.mtu);
            println!("    RX:   {} packets, {} bytes", iface.rx_packets, iface.rx_bytes);
            println!("    TX:   {} packets, {} bytes", iface.tx_packets, iface.tx_bytes);
            println!();
        }
    }
}

/// Ping a host
pub fn ping(args: &[&str]) {
    use crate::drivers::net::{self, Ipv4Address};
    
    if args.is_empty() {
        println!("Usage: ping <host>");
        println!("Example: ping 10.0.2.2");
        return;
    }
    
    let host = args[0];
    
    // Parse IP address
    let ip = match net::parse_ipv4(host) {
        Some(ip) => ip,
        None => {
            println!("Invalid IP address: {}", host);
            println!("Note: DNS resolution not yet implemented");
            return;
        }
    };
    
    println!("PING {} ({}):", host, ip);
    
    // Check if we have VirtIO-Net
    if !crate::drivers::virtio::net::is_available() {
        println!("Error: No network interface available");
        println!("Hint: Run with 'make run-arm-net' to enable networking");
        return;
    }
    
    // Create ICMP echo request
    use crate::drivers::net::icmp;
    let data = b"DebOS ping!";
    let icmp_packet = icmp::create_echo_request(1, 1, data);
    
    // Create IPv4 packet
    use crate::drivers::net::ipv4;
    let src_ip = Ipv4Address::new(10, 0, 2, 15); // Our IP
    let ip_packet = ipv4::create_packet(
        src_ip,
        ip,
        ipv4::Protocol::ICMP,
        64, // TTL
        &icmp_packet,
    );
    
    // Create Ethernet frame
    use crate::drivers::net::{ethernet, MacAddress};
    let src_mac = MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
    let dst_mac = MacAddress::BROADCAST; // Would use ARP in real implementation
    let frame = ethernet::create_frame(dst_mac, src_mac, ethernet::EtherType::IPV4, &ip_packet);
    
    // Send packet
    match crate::drivers::virtio::net::send_packet(&frame) {
        Ok(_) => {
            println!("Sent {} bytes to {}", frame.len(), ip);
            println!("(Waiting for reply...)");
            
            // Poll for reply (simple version)
            for _ in 0..100 {
                if let Some(_reply) = crate::drivers::virtio::net::recv_packet() {
                    println!("Reply received from {}", ip);
                    return;
                }
                for _ in 0..10000 {
                    core::hint::spin_loop();
                }
            }
            println!("Request timed out");
        }
        Err(e) => {
            println!("Failed to send: {}", e);
        }
    }
}

/// Show ARP cache
pub fn arp(_args: &[&str]) {
    use crate::drivers::net::arp;
    
    println!("ARP Cache:");
    println!("==========");
    println!();
    println!("{:<20} {:<20}", "IP Address", "MAC Address");
    println!("{:-<20} {:-<20}", "", "");
    
    let entries = arp::list_cache();
    if entries.is_empty() {
        println!("(empty)");
    } else {
        for (ip, mac) in entries {
            println!("{:<20} {:<20}", alloc::format!("{}", ip), alloc::format!("{}", mac));
        }
    }
}

/// Show network statistics
pub fn netstat(_args: &[&str]) {
    use crate::drivers::net;
    use crate::drivers::net::socket;
    
    println!("Network Statistics:");
    println!("===================");
    println!();
    
    // Interface stats
    for name in net::list_interfaces() {
        if let Some(iface) = net::get_interface(&name) {
            println!("{}: RX {} pkts / {} bytes, TX {} pkts / {} bytes",
                name, iface.rx_packets, iface.rx_bytes, 
                iface.tx_packets, iface.tx_bytes);
        }
    }
    
    // VirtIO-Net stats
    if let Some(stats) = crate::drivers::virtio::net::get_stats() {
        println!();
        println!("VirtIO-Net: RX {} pkts / {} bytes, TX {} pkts / {} bytes",
            stats.0, stats.2, stats.1, stats.3);
    }
    
    // Socket info
    let sockets = socket::list_sockets();
    println!();
    println!("Active sockets: {}", sockets.len());
    for fd in sockets {
        if let Some((domain, sock_type, state)) = socket::get_socket(fd) {
            println!("  fd={}: {:?} {:?} {:?}", fd, domain, sock_type, state);
        }
    }
}

// ============================================================================
// Device Commands
// ============================================================================

/// List all devices
pub fn devices(_args: &[&str]) {
    use crate::drivers::device::DEVICE_MANAGER;
    
    println!("Device Tree:");
    println!("============");
    println!();
    
    DEVICE_MANAGER.lock().print_tree();
}

/// List PCI devices
pub fn lspci(_args: &[&str]) {
    println!("PCI Devices:");
    println!("============");
    println!();
    
    // PCI enumeration not yet implemented
    println!("(PCI enumeration not yet implemented)");
    println!();
    println!("On QEMU virt machine, devices are VirtIO-MMIO based.");
    println!("Use 'devices' command to see the device tree.");
}

/// List USB devices
pub fn lsusb(_args: &[&str]) {
    println!("USB Devices:");
    println!("============");
    println!();
    
    // USB enumeration not yet implemented
    println!("(USB enumeration not yet implemented)");
    println!();
    println!("USB support requires xHCI controller driver.");
}
