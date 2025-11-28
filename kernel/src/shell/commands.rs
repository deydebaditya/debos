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
    {
        println!("(Reboot not implemented on AArch64 - please restart QEMU)");
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
        if let Err(e) = fs::mkdir(path) {
            println!("mkdir: cannot create directory '{}': {}", path, e);
        }
    }
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
