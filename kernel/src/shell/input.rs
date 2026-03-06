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
    
    static POLL_COUNT: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    
    let count = POLL_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    
    // Print debug info every ~5 seconds (assuming ~1M polls/sec)
    if count % 5_000_000 == 0 {
        // Read FR register directly for debug
        let fr = unsafe {
            let base = 0x0900_0000 as *const u32;
            base.add(0x18 / 4).read_volatile()
        };
        crate::println!("\n[UART-DBG] polls={}, FR=0x{:08x}, RXFE={}", count, fr, (fr >> 4) & 1);
    }
    
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

