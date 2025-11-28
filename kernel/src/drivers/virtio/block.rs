//! VirtIO-Block Driver
//!
//! Block device driver using VirtIO protocol.
//!
//! ## Device Configuration
//! - Offset 0x00: capacity (u64) - Number of 512-byte sectors
//! - Offset 0x08: size_max (u32) - Max segment size
//! - Offset 0x0C: seg_max (u32) - Max segments per request
//!
//! ## Request Format
//! - Header: type (u32), reserved (u32), sector (u64)
//! - Data: sector data
//! - Status: status byte (0=OK, 1=IOERR, 2=UNSUPPORTED)

use core::sync::atomic::{AtomicBool, Ordering};
use alloc::boxed::Box;
use alloc::vec::Vec;
use spin::Mutex;

use super::mmio::MmioDevice;
use super::queue::VirtQueue;
use crate::drivers::block::{BlockDevice, BlockError};

/// VirtIO block feature flags
mod block_features {
    pub const SIZE_MAX: u64 = 1 << 1;
    pub const SEG_MAX: u64 = 1 << 2;
    pub const GEOMETRY: u64 = 1 << 4;
    pub const RO: u64 = 1 << 5;
    pub const BLK_SIZE: u64 = 1 << 6;
    pub const FLUSH: u64 = 1 << 9;
    pub const TOPOLOGY: u64 = 1 << 10;
    pub const CONFIG_WCE: u64 = 1 << 11;
    pub const DISCARD: u64 = 1 << 13;
    pub const WRITE_ZEROES: u64 = 1 << 14;
}

/// Request types
mod request_type {
    pub const IN: u32 = 0;       // Read
    pub const OUT: u32 = 1;      // Write
    pub const FLUSH: u32 = 4;    // Flush
    pub const DISCARD: u32 = 11;
    pub const WRITE_ZEROES: u32 = 13;
}

/// Request status
mod request_status {
    pub const OK: u8 = 0;
    pub const IOERR: u8 = 1;
    pub const UNSUPP: u8 = 2;
}

/// Block request header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct BlockRequestHeader {
    req_type: u32,
    reserved: u32,
    sector: u64,
}

/// A VirtIO block device
pub struct VirtioBlockDevice {
    /// MMIO device handle
    device: MmioDevice,
    /// Request queue
    queue: Mutex<VirtQueue>,
    /// Device capacity in sectors
    capacity: u64,
    /// Block size (usually 512)
    block_size: u32,
    /// Read-only flag
    read_only: bool,
    /// Request buffers pool
    buffers: Mutex<Vec<Box<RequestBuffer>>>,
}

/// Buffer for a block request
#[repr(C, align(16))]
struct RequestBuffer {
    header: BlockRequestHeader,
    data: [u8; 512],
    status: u8,
}

impl RequestBuffer {
    fn new() -> Self {
        RequestBuffer {
            header: BlockRequestHeader {
                req_type: 0,
                reserved: 0,
                sector: 0,
            },
            data: [0; 512],
            status: 0xFF,
        }
    }
}

/// Global block device
static BLOCK_DEVICE: Mutex<Option<VirtioBlockDevice>> = Mutex::new(None);

/// Pending operation flag
static OPERATION_PENDING: AtomicBool = AtomicBool::new(false);

/// Initialize a VirtIO block device
pub fn init_device(device: &MmioDevice) -> Result<(), &'static str> {
    let mut device = MmioDevice::new(device.base_addr(), device.irq())
        .ok_or("Failed to create device")?;
    
    // Initialize device with required features
    let required = block_features::BLK_SIZE;
    let features = device.init_device(required)?;
    
    // Read device configuration
    let capacity = device.read_config_u64(0);
    let block_size = if features & block_features::BLK_SIZE != 0 {
        device.read_config_u32(20)
    } else {
        512
    };
    let read_only = features & block_features::RO != 0;
    
    crate::println!("    VirtIO-Block: {} sectors, {} bytes/sector{}",
        capacity, block_size, if read_only { " (read-only)" } else { "" });
    
    // Set up request queue
    let queue_size = device.queue_max_size(0);
    if queue_size == 0 {
        return Err("No queue available");
    }
    
    let queue = VirtQueue::new(0, queue_size.min(64));
    device.setup_queue(&queue);
    
    // Finish initialization
    device.finish_init();
    
    // Create request buffer pool
    let mut buffers = Vec::with_capacity(8);
    for _ in 0..8 {
        buffers.push(Box::new(RequestBuffer::new()));
    }
    
    // Store the device
    let block_dev = VirtioBlockDevice {
        device,
        queue: Mutex::new(queue),
        capacity,
        block_size,
        read_only,
        buffers: Mutex::new(buffers),
    };
    
    *BLOCK_DEVICE.lock() = Some(block_dev);
    
    Ok(())
}

impl VirtioBlockDevice {
    /// Read a sector from the device
    fn read_sector_internal(&self, sector: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        if sector >= self.capacity {
            return Err(BlockError::InvalidSector);
        }
        
        if buf.len() < self.block_size as usize {
            return Err(BlockError::BufferTooSmall);
        }
        
        // Get a request buffer
        let mut req_buf = self.buffers.lock().pop()
            .ok_or(BlockError::NoBuffers)?;
        
        // Set up the request
        req_buf.header.req_type = request_type::IN;
        req_buf.header.reserved = 0;
        req_buf.header.sector = sector;
        req_buf.status = 0xFF;
        
        // Build descriptor chain
        let header_addr = &req_buf.header as *const _ as u64;
        let data_addr = req_buf.data.as_ptr() as u64;
        let status_addr = &req_buf.status as *const _ as u64;
        
        let buffers = [
            (header_addr, 16, false),      // Header (device reads)
            (data_addr, 512, true),        // Data (device writes)
            (status_addr, 1, true),        // Status (device writes)
        ];
        
        // Add to queue
        {
            let mut queue = self.queue.lock();
            queue.add_buffer_chain(&buffers).map_err(|_| BlockError::QueueFull)?;
        }
        
        // Notify device
        self.device.notify_queue(0);
        
        // Wait for completion (polling)
        loop {
            let mut queue = self.queue.lock();
            if let Some((_head, _len)) = queue.poll_used() {
                break;
            }
            drop(queue);
            
            // Small delay
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }
        
        // Check status
        if req_buf.status != request_status::OK {
            self.buffers.lock().push(req_buf);
            return Err(BlockError::IoError);
        }
        
        // Copy data
        buf[..512].copy_from_slice(&req_buf.data);
        
        // Return buffer to pool
        self.buffers.lock().push(req_buf);
        
        Ok(())
    }
    
    /// Write a sector to the device
    fn write_sector_internal(&self, sector: u64, data: &[u8]) -> Result<(), BlockError> {
        if self.read_only {
            return Err(BlockError::ReadOnly);
        }
        
        if sector >= self.capacity {
            return Err(BlockError::InvalidSector);
        }
        
        if data.len() < self.block_size as usize {
            return Err(BlockError::BufferTooSmall);
        }
        
        // Get a request buffer
        let mut req_buf = self.buffers.lock().pop()
            .ok_or(BlockError::NoBuffers)?;
        
        // Set up the request
        req_buf.header.req_type = request_type::OUT;
        req_buf.header.reserved = 0;
        req_buf.header.sector = sector;
        req_buf.data.copy_from_slice(&data[..512]);
        req_buf.status = 0xFF;
        
        // Build descriptor chain
        let header_addr = &req_buf.header as *const _ as u64;
        let data_addr = req_buf.data.as_ptr() as u64;
        let status_addr = &req_buf.status as *const _ as u64;
        
        let buffers = [
            (header_addr, 16, false),      // Header (device reads)
            (data_addr, 512, false),       // Data (device reads)
            (status_addr, 1, true),        // Status (device writes)
        ];
        
        // Add to queue
        {
            let mut queue = self.queue.lock();
            queue.add_buffer_chain(&buffers).map_err(|_| BlockError::QueueFull)?;
        }
        
        // Notify device
        self.device.notify_queue(0);
        
        // Wait for completion (polling)
        loop {
            let mut queue = self.queue.lock();
            if let Some((_head, _len)) = queue.poll_used() {
                break;
            }
            drop(queue);
            
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }
        
        // Check status
        let status = req_buf.status;
        self.buffers.lock().push(req_buf);
        
        if status != request_status::OK {
            return Err(BlockError::IoError);
        }
        
        Ok(())
    }
}

/// Get the block device if available
pub fn get_device() -> Option<&'static Mutex<Option<VirtioBlockDevice>>> {
    Some(&BLOCK_DEVICE)
}

/// Read a sector from the block device
pub fn read_sector(sector: u64, buf: &mut [u8]) -> Result<(), BlockError> {
    let device = BLOCK_DEVICE.lock();
    let dev = device.as_ref().ok_or(BlockError::NoDevice)?;
    dev.read_sector_internal(sector, buf)
}

/// Write a sector to the block device
pub fn write_sector(sector: u64, data: &[u8]) -> Result<(), BlockError> {
    let device = BLOCK_DEVICE.lock();
    let dev = device.as_ref().ok_or(BlockError::NoDevice)?;
    dev.write_sector_internal(sector, data)
}

/// Get device information
pub fn get_info() -> Option<(u64, u32, bool)> {
    let device = BLOCK_DEVICE.lock();
    device.as_ref().map(|d| (d.capacity, d.block_size, d.read_only))
}

