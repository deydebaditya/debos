//! GICv2 - Generic Interrupt Controller Driver
//!
//! The GIC handles interrupt distribution on AArch64.
//! QEMU virt machine uses GICv2 at:
//! - Distributor: 0x0800_0000
//! - CPU Interface: 0x0801_0000

use spin::Mutex;

/// GIC Distributor base address (QEMU virt)
const GICD_BASE: usize = 0x0800_0000;

/// GIC CPU Interface base address (QEMU virt)  
const GICC_BASE: usize = 0x0801_0000;

/// Distributor registers
mod gicd {
    pub const CTLR: usize = 0x000;      // Control Register
    pub const TYPER: usize = 0x004;     // Type Register
    pub const ISENABLER: usize = 0x100; // Interrupt Set-Enable
    pub const ICENABLER: usize = 0x180; // Interrupt Clear-Enable
    pub const IPRIORITYR: usize = 0x400; // Interrupt Priority
    pub const ITARGETSR: usize = 0x800;  // Interrupt Target
    pub const ICFGR: usize = 0xC00;      // Interrupt Configuration
}

/// CPU Interface registers
mod gicc {
    pub const CTLR: usize = 0x000;      // Control Register
    pub const PMR: usize = 0x004;       // Priority Mask Register
    pub const IAR: usize = 0x00C;       // Interrupt Acknowledge
    pub const EOIR: usize = 0x010;      // End of Interrupt
}

/// Timer interrupt (from ARM architectural timer)
pub const TIMER_IRQ: u32 = 30;

/// UART interrupt
pub const UART_IRQ: u32 = 33;

/// GIC state
pub struct Gic {
    gicd_base: usize,
    gicc_base: usize,
}

impl Gic {
    /// Create a new GIC instance
    pub const fn new(gicd_base: usize, gicc_base: usize) -> Self {
        Gic { gicd_base, gicc_base }
    }
    
    /// Initialize the GIC
    pub fn init(&mut self) {
        unsafe {
            let gicd = self.gicd_base as *mut u32;
            let gicc = self.gicc_base as *mut u32;
            
            // Disable distributor
            gicd.add(gicd::CTLR / 4).write_volatile(0);
            
            // Set all interrupts to target CPU 0
            // Each ITARGETSR register covers 4 interrupts (1 byte each)
            for i in 8..32 {
                gicd.add((gicd::ITARGETSR + i * 4) / 4).write_volatile(0x01010101);
            }
            
            // Set all interrupts to lowest priority
            for i in 8..32 {
                gicd.add((gicd::IPRIORITYR + i * 4) / 4).write_volatile(0xA0A0A0A0);
            }
            
            // Enable distributor
            gicd.add(gicd::CTLR / 4).write_volatile(1);
            
            // Set priority mask to allow all priorities
            gicc.add(gicc::PMR / 4).write_volatile(0xFF);
            
            // Enable CPU interface
            gicc.add(gicc::CTLR / 4).write_volatile(1);
        }
    }
    
    /// Enable an interrupt
    pub fn enable_interrupt(&mut self, irq: u32) {
        unsafe {
            let gicd = self.gicd_base as *mut u32;
            let reg = irq / 32;
            let bit = irq % 32;
            
            let addr = gicd.add((gicd::ISENABLER + reg as usize * 4) / 4);
            addr.write_volatile(1 << bit);
        }
    }
    
    /// Disable an interrupt
    pub fn disable_interrupt(&mut self, irq: u32) {
        unsafe {
            let gicd = self.gicd_base as *mut u32;
            let reg = irq / 32;
            let bit = irq % 32;
            
            let addr = gicd.add((gicd::ICENABLER + reg as usize * 4) / 4);
            addr.write_volatile(1 << bit);
        }
    }
    
    /// Acknowledge an interrupt (get pending IRQ)
    pub fn acknowledge(&mut self) -> u32 {
        unsafe {
            let gicc = self.gicc_base as *mut u32;
            gicc.add(gicc::IAR / 4).read_volatile()
        }
    }
    
    /// Signal end of interrupt
    pub fn end_interrupt(&mut self, irq: u32) {
        unsafe {
            let gicc = self.gicc_base as *mut u32;
            gicc.add(gicc::EOIR / 4).write_volatile(irq);
        }
    }
}

/// Global GIC instance
pub static GIC: Mutex<Gic> = Mutex::new(Gic::new(GICD_BASE, GICC_BASE));

/// Initialize the GIC
pub fn init() {
    let mut gic = GIC.lock();
    gic.init();
    
    // Enable timer interrupt
    gic.enable_interrupt(TIMER_IRQ);
    
    // Enable UART RX interrupt
    gic.enable_interrupt(UART_IRQ);
    
    crate::println!("[OK] GIC initialized");
}

/// Handle an interrupt from the GIC
pub fn handle_interrupt() {
    let mut gic = GIC.lock();
    let irq = gic.acknowledge();
    
    // Check for spurious interrupt
    if irq >= 1020 {
        return;
    }
    
    match irq {
        TIMER_IRQ => {
            // Timer interrupt - call scheduler
            drop(gic); // Release lock before calling scheduler
            crate::scheduler::on_timer_tick();
            GIC.lock().end_interrupt(irq);
        }
        UART_IRQ => {
            drop(gic);
            crate::println!("[GIC] UART IRQ received!");
            super::uart::handle_rx_interrupt();
            GIC.lock().end_interrupt(irq);
        }
        _ => {
            crate::println!("[IRQ] Unknown interrupt: {}", irq);
            gic.end_interrupt(irq);
        }
    }
}

/// Enable the ARM architectural timer
pub fn enable_timer() {
    unsafe {
        // Set timer compare value (e.g., 10ms at 62.5MHz)
        let freq: u64;
        core::arch::asm!("mrs {}, CNTFRQ_EL0", out(reg) freq);
        
        let interval = freq / 100; // 10ms intervals
        
        core::arch::asm!("msr CNTP_TVAL_EL0, {}", in(reg) interval);
        
        // Enable timer
        core::arch::asm!("msr CNTP_CTL_EL0, {}", in(reg) 1u64);
    }
    
    crate::println!("[OK] Timer enabled");
}

