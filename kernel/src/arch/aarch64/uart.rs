//! PL011 UART Driver for AArch64
//!
//! Provides serial I/O on QEMU virt machine.
//! The PL011 UART is memory-mapped at 0x0900_0000, IRQ 33.
//!
//! RX uses interrupt-driven buffering: PL011 fires IRQ on receive,
//! the handler drains the FIFO into a lock-free ring buffer,
//! and the shell reads from the buffer.

use core::fmt;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

const UART_BASE: usize = 0x0900_0000;

mod regs {
    pub const DR: usize = 0x00;
    pub const RSR: usize = 0x04;
    pub const FR: usize = 0x18;
    pub const IBRD: usize = 0x24;
    pub const FBRD: usize = 0x28;
    pub const LCR_H: usize = 0x2C;
    pub const CR: usize = 0x30;
    pub const IFLS: usize = 0x34;    // Interrupt FIFO Level Select
    pub const IMSC: usize = 0x38;
    pub const MIS: usize = 0x40;     // Masked Interrupt Status
    pub const ICR: usize = 0x44;
}

mod flags {
    pub const TXFF: u32 = 1 << 5;
    pub const RXFE: u32 = 1 << 4;
}

/// IMSC bits
const RXIM: u32 = 1 << 4;   // Receive interrupt mask
const RTIM: u32 = 1 << 6;   // Receive timeout interrupt mask

// --------------- Lock-free RX ring buffer ---------------

const RX_BUF_SIZE: usize = 256;

static RX_BUF: [core::sync::atomic::AtomicU8; RX_BUF_SIZE] = {
    const ZERO: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);
    [ZERO; RX_BUF_SIZE]
};
static RX_HEAD: AtomicUsize = AtomicUsize::new(0); // written by IRQ handler
static RX_TAIL: AtomicUsize = AtomicUsize::new(0); // read by consumer

/// Push a byte into the RX ring buffer (called from IRQ handler only)
fn rx_push(byte: u8) {
    let head = RX_HEAD.load(Ordering::Relaxed);
    let next = (head + 1) % RX_BUF_SIZE;
    if next != RX_TAIL.load(Ordering::Acquire) {
        RX_BUF[head].store(byte, Ordering::Relaxed);
        RX_HEAD.store(next, Ordering::Release);
    }
    // else: buffer full, drop the byte
}

/// Pop a byte from the RX ring buffer (called from shell/consumer)
pub fn rx_pop() -> Option<u8> {
    let tail = RX_TAIL.load(Ordering::Relaxed);
    if tail == RX_HEAD.load(Ordering::Acquire) {
        return None;
    }
    let byte = RX_BUF[tail].load(Ordering::Relaxed);
    RX_TAIL.store((tail + 1) % RX_BUF_SIZE, Ordering::Release);
    Some(byte)
}

/// Called from the GIC UART IRQ handler -- drain PL011 FIFO into ring buffer
pub fn handle_rx_interrupt() {
    unsafe {
        let base = UART_BASE as *mut u32;

        // Drain all available bytes from the hardware FIFO
        loop {
            let fr = base.add(regs::FR / 4).read_volatile();
            if (fr & flags::RXFE) != 0 {
                break;
            }
            let data = base.add(regs::DR / 4).read_volatile();
            if (data & 0xF00) != 0 {
                // Error -- clear and skip
                base.add(regs::RSR / 4).write_volatile(0);
                continue;
            }
            rx_push((data & 0xFF) as u8);
        }

        // Clear RX + RT interrupt flags
        base.add(regs::ICR / 4).write_volatile(RXIM | RTIM);
    }
}

// --------------- UART driver (TX + init) ---------------

pub struct Uart {
    base: usize,
}

impl Uart {
    pub const fn new(base: usize) -> Self {
        Uart { base }
    }

    pub fn init(&mut self) {
        unsafe {
            let base = self.base as *mut u32;

            // Disable UART while configuring
            base.add(regs::CR / 4).write_volatile(0);

            // Clear all pending interrupts
            base.add(regs::ICR / 4).write_volatile(0x7FF);

            // Baud rate 115200 with 24 MHz clock
            base.add(regs::IBRD / 4).write_volatile(13);
            base.add(regs::FBRD / 4).write_volatile(1);

            // 8N1, FIFO enabled
            base.add(regs::LCR_H / 4).write_volatile(0x70);

            // RX FIFO trigger at 1/8 full (earliest possible interrupt)
            base.add(regs::IFLS / 4).write_volatile(0x00);

            // Enable RX and RX-timeout interrupts
            base.add(regs::IMSC / 4).write_volatile(RXIM | RTIM);

            // Enable UART, TX, RX
            base.add(regs::CR / 4).write_volatile(0x301);
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        unsafe {
            let base = self.base as *mut u32;
            while (base.add(regs::FR / 4).read_volatile() & flags::TXFF) != 0 {
                core::hint::spin_loop();
            }
            base.add(regs::DR / 4).write_volatile(byte as u32);
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

/// Global UART instance (used for TX / init only)
pub static UART: Mutex<Uart> = Mutex::new(Uart::new(UART_BASE));

pub fn init() {
    UART.lock().init();
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    let enabled = super::disable_interrupts();
    UART.lock().write_fmt(args).unwrap();
    super::restore_interrupts(enabled);
}

