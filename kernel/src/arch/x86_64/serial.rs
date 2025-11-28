//! Serial Port Output (COM1)
//!
//! Provides early console output via the serial port for debugging.
//! This is essential for kernel development as it works before any
//! graphics initialization.

use spin::Mutex;
use uart_16550::SerialPort;
use core::fmt;
use lazy_static::lazy_static;

lazy_static! {
    /// Global serial port instance for COM1 (0x3F8)
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

/// Initialize serial port
pub fn init() {
    // Serial port is initialized via lazy_static when first accessed
    // Force initialization by acquiring lock
    let _ = SERIAL1.lock();
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    
    // Disable interrupts to prevent deadlock
    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed");
    });
}
