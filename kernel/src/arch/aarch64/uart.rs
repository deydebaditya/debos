//! PL011 UART Driver for AArch64
//!
//! Provides serial output for debugging on QEMU virt machine.
//! The PL011 UART is memory-mapped at 0x0900_0000.

use core::fmt;
use spin::Mutex;

/// PL011 UART base address on QEMU virt machine
const UART_BASE: usize = 0x0900_0000;

/// PL011 Register offsets
mod regs {
    pub const DR: usize = 0x00;      // Data Register
    pub const FR: usize = 0x18;      // Flag Register
    pub const IBRD: usize = 0x24;    // Integer Baud Rate
    pub const FBRD: usize = 0x28;    // Fractional Baud Rate
    pub const LCR_H: usize = 0x2C;   // Line Control Register
    pub const CR: usize = 0x30;      // Control Register
    pub const IMSC: usize = 0x38;    // Interrupt Mask Set/Clear
    pub const ICR: usize = 0x44;     // Interrupt Clear Register
}

/// Flag Register bits
mod flags {
    pub const TXFF: u32 = 1 << 5;    // Transmit FIFO Full
    pub const RXFE: u32 = 1 << 4;    // Receive FIFO Empty
}

/// PL011 UART driver
pub struct Uart {
    base: usize,
}

impl Uart {
    /// Create a new UART instance
    pub const fn new(base: usize) -> Self {
        Uart { base }
    }
    
    /// Initialize the UART
    pub fn init(&mut self) {
        unsafe {
            let base = self.base as *mut u32;
            
            // Disable UART
            base.add(regs::CR / 4).write_volatile(0);
            
            // Clear pending interrupts
            base.add(regs::ICR / 4).write_volatile(0x7FF);
            
            // Set baud rate (115200 with 24MHz clock)
            // Divisor = 24000000 / (16 * 115200) = 13.0208
            // Integer part = 13, Fractional part = 0.0208 * 64 = 1
            base.add(regs::IBRD / 4).write_volatile(13);
            base.add(regs::FBRD / 4).write_volatile(1);
            
            // 8 bits, no parity, 1 stop bit, FIFO enabled
            base.add(regs::LCR_H / 4).write_volatile(0x70);
            
            // Mask all interrupts
            base.add(regs::IMSC / 4).write_volatile(0);
            
            // Enable UART, TX, RX
            // CR bits: bit 0 = UARTEN, bit 8 = TXE, bit 9 = RXE
            // 0x301 = 0b0000_0011_0000_0001 = UARTEN | TXE | RXE
            base.add(regs::CR / 4).write_volatile(0x301);
            
            // Ensure RX is enabled by reading status
            let _ = base.add(regs::FR / 4).read_volatile();
        }
    }
    
    /// Write a byte to the UART
    pub fn write_byte(&mut self, byte: u8) {
        unsafe {
            let base = self.base as *mut u32;
            
            // Wait until TX FIFO is not full
            while (base.add(regs::FR / 4).read_volatile() & flags::TXFF) != 0 {
                core::hint::spin_loop();
            }
            
            // Write the byte
            base.add(regs::DR / 4).write_volatile(byte as u32);
        }
    }
    
    /// Read a byte from the UART (if available)
    pub fn read_byte(&mut self) -> Option<u8> {
        unsafe {
            let base = self.base as *mut u32;
            
            // Read Flag Register to check status
            let fr = base.add(regs::FR / 4).read_volatile();
            
            // Check if RX FIFO is empty
            if (fr & flags::RXFE) != 0 {
                return None;
            }
            
            // Check for overrun error (bit 3)
            let has_error = (fr & (1 << 3)) != 0;
            if has_error {
                // Clear error by reading data register
                let _ = base.add(regs::DR / 4).read_volatile();
                return None;
            }
            
            // Read the byte from data register
            let data = base.add(regs::DR / 4).read_volatile();
            Some((data & 0xFF) as u8)
        }
    }
    
    /// Check if UART has data available (for debugging)
    pub fn has_data(&mut self) -> bool {
        unsafe {
            let base = self.base as *mut u32;
            let fr = base.add(regs::FR / 4).read_volatile();
            (fr & flags::RXFE) == 0
        }
    }
}

impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
        Ok(())
    }
}

/// Global UART instance
pub static UART: Mutex<Uart> = Mutex::new(Uart::new(UART_BASE));

/// Initialize the UART
pub fn init() {
    UART.lock().init();
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    
    // Disable interrupts while printing to prevent deadlock
    let enabled = super::disable_interrupts();
    UART.lock().write_fmt(args).unwrap();
    super::restore_interrupts(enabled);
}

