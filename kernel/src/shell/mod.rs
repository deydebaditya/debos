//! DebOS Kernel Shell
//!
//! A simple interactive shell for kernel debugging and system interaction.
//! This runs as a kernel thread until we have full userspace support.

mod commands;
mod input;
pub(crate) mod sdk;

use alloc::string::String;
use alloc::vec::Vec;
use crate::println;

/// Maximum command line length
const MAX_LINE_LENGTH: usize = 256;

/// The kernel shell
pub struct Shell {
    /// Command history
    history: Vec<String>,
    /// Current input buffer
    input_buffer: String,
    /// Whether the shell is running
    running: bool,
    /// Current working directory (cached for prompt display)
    current_dir: String,
}

impl Shell {
    /// Create a new shell instance
    pub fn new() -> Self {
        // Initialize SDK utilities (safe credential access)
        // Do this first, but don't block if scheduler isn't ready
        sdk::init();
        
        // Get current working directory
        // Use fallback if getcwd fails (shouldn't happen, but be safe)
        let current_dir = crate::fs::getcwd().unwrap_or_else(|_| String::from("/"));
        
        Shell {
            history: Vec::new(),
            input_buffer: String::with_capacity(MAX_LINE_LENGTH),
            running: true,
            current_dir,
        }
    }
    
    /// Run the shell main loop
    pub fn run(&mut self) {
        self.print_banner();
        
        while self.running {
            self.print_prompt();
            
            // Read a line of input
            match self.read_line() {
                Some(line) => {
                    let trimmed = line.trim();
                    
                    if !trimmed.is_empty() {
                        // Add to history
                        self.history.push(line.clone());
                        
                        // Execute the command
                        self.execute(trimmed);
                        
                        // Ensure we end on a new line after command execution
                        // This is critical for commands that don't output anything (mkdir, touch, etc.)
                        crate::print!("\n");
                    } else {
                        // Empty line - just print a newline and continue
                        crate::print!("\n");
                    }
                }
                None => {
                    // Ctrl+C or Ctrl+D was pressed
                    // For Ctrl+C, continue loop to show new prompt
                    // For Ctrl+D, self.running will be false and we'll exit
                    continue;
                }
            }
        }
        
        println!("Shell exited.");
    }
    
    /// Print the shell banner
    fn print_banner(&self) {
        println!();
        println!("╔═══════════════════════════════════════════════════════════════╗");
        println!("║                      DebOS Shell v0.1                         ║");
        println!("║              Type 'help' for available commands               ║");
        println!("╚═══════════════════════════════════════════════════════════════╝");
        println!();
    }
    
    /// Print the shell prompt
    fn print_prompt(&mut self) {
        // Use cached current directory - it's updated when cd is called
        // Note: We rely on the newline added after command execution to ensure
        // we're on a fresh line before printing the prompt
        crate::print!("debos ({})> ", self.current_dir);
    }
    
    /// Read a line of input from serial
    fn read_line(&mut self) -> Option<String> {
        self.input_buffer.clear();
        
        loop {
            if let Some(c) = input::read_char() {
                match c {
                    // Enter - submit line
                    b'\r' | b'\n' => {
                        println!();
                        return Some(self.input_buffer.clone());
                    }
                    // Backspace
                    0x7F | 0x08 => {
                        if !self.input_buffer.is_empty() {
                            self.input_buffer.pop();
                            // Erase character on screen
                            crate::print!("\x08 \x08");
                        }
                    }
                    // Ctrl+C - cancel line and return to prompt
                    0x03 => {
                        // Print Ctrl+C indicator on a new line
                        println!("^C");
                        self.input_buffer.clear();
                        // Return None to skip command execution
                        // The loop will continue and print a fresh prompt
                        return None;
                    }
                    // Ctrl+D - exit shell
                    0x04 => {
                        println!("^D");
                        self.running = false;
                        return None;
                    }
                    // Regular printable character
                    c if c >= 0x20 && c < 0x7F => {
                        if self.input_buffer.len() < MAX_LINE_LENGTH {
                            self.input_buffer.push(c as char);
                            crate::print!("{}", c as char);
                        }
                    }
                    // Ignore other control characters
                    _ => {}
                }
            } else {
                // Yield to scheduler so timer interrupts can fire.
                // Timer IRQs cause QEMU vCPU exits, letting QEMU's
                // event loop process stdin and deliver it to the PL011.
                crate::scheduler::yield_now();
            }
        }
    }
    
    /// Execute a command
    fn execute(&mut self, line: &str) {
        // Parse command and arguments
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.is_empty() {
            return;
        }
        
        let command = parts[0];
        let args = &parts[1..];
        
        match command {
            // System commands
            "help" | "?" => commands::help(args),
            "info" | "sysinfo" => commands::sysinfo(args),
            "mem" | "memory" => commands::memory(args),
            "threads" | "ps" => commands::threads(args),
            "echo" => commands::echo(args),
            "clear" | "cls" => commands::clear(args),
            "uptime" => commands::uptime(args),
            "reboot" => commands::reboot(args),
            
            // Block device commands
            "disk" => commands::disk(args),
            "blkread" => commands::blkread(args),
            "mount" => commands::mount(args),
            "fatls" => commands::fatls(args),
            "fatcat" => commands::fatcat(args),
            "fatwrite" => commands::fatwrite(args),
            "fatrm" => commands::fatrm(args),
            
            // Text processing commands
            "head" => commands::head(args),
            "tail" => commands::tail(args),
            "grep" => commands::grep(args),
            "edit" | "vim" | "vi" => commands::edit(args),
            
            // Filesystem commands
            "pwd" => commands::pwd(args),
            "ls" | "dir" => commands::ls(args),
            "cd" => {
                commands::cd(args);
                // Update current directory after cd - use cached value if getcwd fails
                if let Ok(dir) = crate::fs::getcwd() {
                    self.current_dir = dir;
                }
                // If getcwd fails, keep the cached value
            },
            "mkdir" => commands::mkdir(args),
            "rmdir" => commands::rmdir(args),
            "touch" => commands::touch(args),
            "cat" | "type" => commands::cat(args),
            "rm" | "del" => commands::rm(args),
            "write" => commands::write_file(args),
            "stat" => commands::stat_cmd(args),
            "tree" => commands::tree(args),
            
            // User and security commands
            "whoami" => commands::whoami(args),
            "id" => commands::id(args),
            "users" => commands::users_list(args),
            "groups" => commands::groups_list(args),
            "useradd" => commands::useradd(args),
            "userdel" => commands::userdel(args),
            "passwd" => commands::passwd(args),
            "su" => commands::su(args),
            "sudo" => commands::sudo(args),
            "login" => commands::login(args),
            
            // Network commands
            "ifconfig" => commands::ifconfig(args),
            "ping" => commands::ping(args),
            "arp" => commands::arp(args),
            "netstat" => commands::netstat(args),
            
            // Device commands
            "devices" | "lsdev" => commands::devices(args),
            "lspci" => commands::lspci(args),
            "lsusb" => commands::lsusb(args),
            
            // Exit
            "exit" | "quit" | "logout" => {
                self.running = false;
            }
            
            _ => {
                println!("Unknown command: '{}'. Type 'help' for available commands.", command);
            }
        }
    }
}

/// Start the shell as a kernel thread
pub fn start() {
    let mut shell = Shell::new();
    shell.run();
}

/// Entry point for the shell thread
pub extern "C" fn shell_thread_entry() {
    // Debug: Confirm shell thread is running
    crate::println!("[SHELL] Thread entry point reached");
    
    // Ensure interrupts are enabled (safety check)
    #[cfg(target_arch = "aarch64")]
    {
        crate::arch::enable_interrupts();
        crate::println!("[SHELL] Interrupts enabled, starting shell...");
    }
    
    start();
    crate::scheduler::exit_thread(0);
}
