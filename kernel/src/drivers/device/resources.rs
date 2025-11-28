//! Device Resources
//!
//! Tracks hardware resources assigned to devices: MMIO regions, IRQs, DMA buffers, I/O ports.

use alloc::vec::Vec;

/// Memory-Mapped I/O region
#[derive(Debug, Clone)]
pub struct MmioRegion {
    /// Physical base address
    pub phys_base: usize,
    
    /// Virtual base address (after mapping)
    pub virt_base: Option<usize>,
    
    /// Size in bytes
    pub size: usize,
    
    /// Cacheability (false = device memory)
    pub cacheable: bool,
}

impl MmioRegion {
    pub fn new(phys_base: usize, size: usize) -> Self {
        MmioRegion {
            phys_base,
            virt_base: None,
            size,
            cacheable: false,
        }
    }
}

/// I/O Port range (x86 only)
#[derive(Debug, Clone)]
pub struct IoPortRange {
    /// Base I/O port
    pub base: u16,
    
    /// Number of ports
    pub size: u16,
}

impl IoPortRange {
    pub fn new(base: u16, size: u16) -> Self {
        IoPortRange { base, size }
    }
}

/// DMA buffer
#[derive(Debug, Clone)]
pub struct DmaBuffer {
    /// Physical address (for DMA)
    pub phys_addr: usize,
    
    /// Virtual address (for CPU)
    pub virt_addr: usize,
    
    /// Size in bytes
    pub size: usize,
    
    /// Direction
    pub direction: DmaDirection,
}

/// DMA direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    /// Device to memory
    ToDevice,
    
    /// Memory to device
    FromDevice,
    
    /// Bidirectional
    Bidirectional,
}

/// IRQ number
pub type IrqNumber = u32;

/// Device resources
#[derive(Debug, Clone, Default)]
pub struct DeviceResources {
    /// Memory-mapped I/O regions
    pub mmio: Vec<MmioRegion>,
    
    /// Assigned IRQ lines
    pub irqs: Vec<IrqNumber>,
    
    /// DMA buffers
    pub dma: Vec<DmaBuffer>,
    
    /// I/O ports (x86 only)
    pub io_ports: Vec<IoPortRange>,
}

impl DeviceResources {
    /// Create empty resources
    pub fn empty() -> Self {
        DeviceResources {
            mmio: Vec::new(),
            irqs: Vec::new(),
            dma: Vec::new(),
            io_ports: Vec::new(),
        }
    }
    
    /// Add MMIO region
    pub fn add_mmio(&mut self, base: usize, size: usize) {
        self.mmio.push(MmioRegion::new(base, size));
    }
    
    /// Add IRQ
    pub fn add_irq(&mut self, irq: IrqNumber) {
        if !self.irqs.contains(&irq) {
            self.irqs.push(irq);
        }
    }
    
    /// Add I/O port range
    pub fn add_io_port(&mut self, base: u16, size: u16) {
        self.io_ports.push(IoPortRange::new(base, size));
    }
    
    /// Add DMA buffer
    pub fn add_dma(&mut self, phys: usize, virt: usize, size: usize, direction: DmaDirection) {
        self.dma.push(DmaBuffer {
            phys_addr: phys,
            virt_addr: virt,
            size,
            direction,
        });
    }
    
    /// Get first MMIO region
    pub fn first_mmio(&self) -> Option<&MmioRegion> {
        self.mmio.first()
    }
    
    /// Get first IRQ
    pub fn first_irq(&self) -> Option<IrqNumber> {
        self.irqs.first().copied()
    }
    
    /// Get first I/O port range
    pub fn first_io_port(&self) -> Option<&IoPortRange> {
        self.io_ports.first()
    }
}

