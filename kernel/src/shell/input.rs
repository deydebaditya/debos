//! Input handling for the shell
//!
//! Reads characters from serial/UART input.

/// Read a single character from input (non-blocking)
/// Returns None if no character is available
#[cfg(target_arch = "x86_64")]
pub fn read_char() -> Option<u8> {
    use x86_64::instructions::port::Port;
    
    // Check if data is available on COM1
    let mut status_port = Port::<u8>::new(0x3FD);
    let status = unsafe { status_port.read() };
    
    if status & 0x01 != 0 {
        // Data available, read it
        let mut data_port = Port::<u8>::new(0x3F8);
        Some(unsafe { data_port.read() })
    } else {
        None
    }
}

#[cfg(target_arch = "aarch64")]
pub fn read_char() -> Option<u8> {
    use crate::arch::aarch64::uart::UART;
    
    UART.lock().read_byte()
}

/// Read a character, blocking until one is available
#[allow(dead_code)]
pub fn read_char_blocking() -> u8 {
    loop {
        if let Some(c) = read_char() {
            return c;
        }
        // Yield to other threads while waiting
        crate::scheduler::yield_now();
    }
}

