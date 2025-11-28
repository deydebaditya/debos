//! USB Mass Storage Class (MSC) Driver
//!
//! Implements USB Mass Storage for external drives, USB flash drives, etc.
//!
//! ## Transport Protocols
//! - Bulk-Only Transport (BOT) - Most common
//! - Control/Bulk/Interrupt (CBI) - Legacy, not implemented
//!
//! ## Command Sets
//! - SCSI Transparent Command Set
//! - UFI (USB Floppy Interface) - Legacy, not implemented

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use spin::Mutex;
use lazy_static::lazy_static;

use super::{UsbDevice, UsbClass};
use crate::drivers::block::BlockDevice;

/// MSC subclass codes
pub mod MscSubclass {
    pub const SCSI_NOT_REPORTED: u8 = 0x00;
    pub const RBC: u8 = 0x01;  // Reduced Block Commands
    pub const MMC5: u8 = 0x02; // CD/DVD
    pub const UFI: u8 = 0x04;  // Floppy
    pub const SCSI: u8 = 0x06; // SCSI Transparent Command Set
    pub const LSDFS: u8 = 0x07; // LSD FS
    pub const IEEE1667: u8 = 0x08;
}

/// MSC protocol codes
pub mod MscProtocol {
    pub const CBI_INT: u8 = 0x00;
    pub const CBI_NO_INT: u8 = 0x01;
    pub const BBB: u8 = 0x50;  // Bulk-Only Transport
}

/// Command Block Wrapper (CBW)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CommandBlockWrapper {
    /// Signature (0x43425355 = "USBC")
    pub signature: u32,
    /// Tag (matches CSW)
    pub tag: u32,
    /// Transfer length
    pub data_length: u32,
    /// Flags (bit 7 = direction: 0=OUT, 1=IN)
    pub flags: u8,
    /// LUN
    pub lun: u8,
    /// Command length (1-16)
    pub cb_length: u8,
    /// Command block (SCSI command)
    pub cb: [u8; 16],
}

impl CommandBlockWrapper {
    pub const SIGNATURE: u32 = 0x43425355;
    pub const SIZE: usize = 31;
    
    pub fn new(tag: u32, data_length: u32, direction_in: bool, lun: u8, command: &[u8]) -> Self {
        let mut cb = [0u8; 16];
        let len = command.len().min(16);
        cb[..len].copy_from_slice(&command[..len]);
        
        CommandBlockWrapper {
            signature: Self::SIGNATURE,
            tag,
            data_length,
            flags: if direction_in { 0x80 } else { 0x00 },
            lun,
            cb_length: len as u8,
            cb,
        }
    }
}

/// Command Status Wrapper (CSW)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CommandStatusWrapper {
    /// Signature (0x53425355 = "USBS")
    pub signature: u32,
    /// Tag (matches CBW)
    pub tag: u32,
    /// Data residue
    pub data_residue: u32,
    /// Status
    pub status: u8,
}

impl CommandStatusWrapper {
    pub const SIGNATURE: u32 = 0x53425355;
    pub const SIZE: usize = 13;
    
    pub const STATUS_PASSED: u8 = 0x00;
    pub const STATUS_FAILED: u8 = 0x01;
    pub const STATUS_PHASE_ERROR: u8 = 0x02;
}

/// SCSI Commands
pub mod ScsiCommand {
    pub const TEST_UNIT_READY: u8 = 0x00;
    pub const REQUEST_SENSE: u8 = 0x03;
    pub const INQUIRY: u8 = 0x12;
    pub const MODE_SENSE_6: u8 = 0x1A;
    pub const START_STOP_UNIT: u8 = 0x1B;
    pub const READ_CAPACITY_10: u8 = 0x25;
    pub const READ_10: u8 = 0x28;
    pub const WRITE_10: u8 = 0x2A;
    pub const READ_CAPACITY_16: u8 = 0x9E;
    pub const READ_16: u8 = 0x88;
    pub const WRITE_16: u8 = 0x8A;
}

/// SCSI Inquiry Response
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct InquiryResponse {
    pub peripheral: u8,
    pub removable: u8,
    pub version: u8,
    pub response_format: u8,
    pub additional_length: u8,
    pub flags: [u8; 3],
    pub vendor: [u8; 8],
    pub product: [u8; 16],
    pub revision: [u8; 4],
}

impl InquiryResponse {
    pub fn vendor_string(&self) -> String {
        String::from_utf8_lossy(&self.vendor).trim().to_string()
    }
    
    pub fn product_string(&self) -> String {
        String::from_utf8_lossy(&self.product).trim().to_string()
    }
    
    pub fn is_removable(&self) -> bool {
        self.removable & 0x80 != 0
    }
}

/// Read Capacity (10) Response
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ReadCapacity10Response {
    pub last_lba: u32,  // Big-endian
    pub block_size: u32, // Big-endian
}

impl ReadCapacity10Response {
    pub fn last_lba(&self) -> u32 {
        u32::from_be(self.last_lba)
    }
    
    pub fn block_size(&self) -> u32 {
        u32::from_be(self.block_size)
    }
    
    pub fn total_blocks(&self) -> u64 {
        self.last_lba() as u64 + 1
    }
    
    pub fn capacity_bytes(&self) -> u64 {
        self.total_blocks() * self.block_size() as u64
    }
}

/// USB Mass Storage Device
pub struct MscDevice {
    /// USB device info
    pub usb_device: UsbDevice,
    
    /// Interface number
    pub interface: u8,
    
    /// Bulk IN endpoint
    pub bulk_in: u8,
    
    /// Bulk OUT endpoint
    pub bulk_out: u8,
    
    /// Max LUN
    pub max_lun: u8,
    
    /// Block size
    pub block_size: u32,
    
    /// Total blocks
    pub total_blocks: u64,
    
    /// Vendor string
    pub vendor: String,
    
    /// Product string
    pub product: String,
    
    /// Is removable
    pub removable: bool,
    
    /// Command tag counter
    tag: u32,
}

impl MscDevice {
    pub fn new(usb_device: UsbDevice, interface: u8, bulk_in: u8, bulk_out: u8) -> Self {
        MscDevice {
            usb_device,
            interface,
            bulk_in,
            bulk_out,
            max_lun: 0,
            block_size: 512,
            total_blocks: 0,
            vendor: String::new(),
            product: String::new(),
            removable: false,
            tag: 1,
        }
    }
    
    /// Get next command tag
    fn next_tag(&mut self) -> u32 {
        let tag = self.tag;
        self.tag = self.tag.wrapping_add(1);
        tag
    }
    
    /// Build TEST UNIT READY command
    pub fn build_test_unit_ready(&mut self) -> CommandBlockWrapper {
        CommandBlockWrapper::new(
            self.next_tag(),
            0,
            false,
            0,
            &[ScsiCommand::TEST_UNIT_READY, 0, 0, 0, 0, 0],
        )
    }
    
    /// Build INQUIRY command
    pub fn build_inquiry(&mut self) -> CommandBlockWrapper {
        CommandBlockWrapper::new(
            self.next_tag(),
            36,
            true,
            0,
            &[ScsiCommand::INQUIRY, 0, 0, 0, 36, 0],
        )
    }
    
    /// Build READ CAPACITY (10) command
    pub fn build_read_capacity(&mut self) -> CommandBlockWrapper {
        CommandBlockWrapper::new(
            self.next_tag(),
            8,
            true,
            0,
            &[ScsiCommand::READ_CAPACITY_10, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        )
    }
    
    /// Build READ (10) command
    pub fn build_read_10(&mut self, lba: u32, blocks: u16) -> CommandBlockWrapper {
        let lba_bytes = lba.to_be_bytes();
        let len_bytes = blocks.to_be_bytes();
        
        CommandBlockWrapper::new(
            self.next_tag(),
            blocks as u32 * self.block_size,
            true,
            0,
            &[
                ScsiCommand::READ_10,
                0,
                lba_bytes[0], lba_bytes[1], lba_bytes[2], lba_bytes[3],
                0,
                len_bytes[0], len_bytes[1],
                0,
            ],
        )
    }
    
    /// Build WRITE (10) command
    pub fn build_write_10(&mut self, lba: u32, blocks: u16) -> CommandBlockWrapper {
        let lba_bytes = lba.to_be_bytes();
        let len_bytes = blocks.to_be_bytes();
        
        CommandBlockWrapper::new(
            self.next_tag(),
            blocks as u32 * self.block_size,
            false,
            0,
            &[
                ScsiCommand::WRITE_10,
                0,
                lba_bytes[0], lba_bytes[1], lba_bytes[2], lba_bytes[3],
                0,
                len_bytes[0], len_bytes[1],
                0,
            ],
        )
    }
    
    /// Get capacity in bytes
    pub fn capacity_bytes(&self) -> u64 {
        self.total_blocks * self.block_size as u64
    }
    
    /// Get capacity in MB
    pub fn capacity_mb(&self) -> u64 {
        self.capacity_bytes() / (1024 * 1024)
    }
}

impl BlockDevice for MscDevice {
    fn sector_size(&self) -> u32 {
        self.block_size
    }
    
    fn sector_count(&self) -> u64 {
        self.total_blocks
    }
    
    fn is_read_only(&self) -> bool {
        false // MSC devices are typically writable
    }
    
    fn read_sector(&self, sector: u64, buffer: &mut [u8]) -> Result<(), crate::drivers::block::BlockError> {
        if sector >= self.total_blocks {
            return Err(crate::drivers::block::BlockError::InvalidSector);
        }
        if buffer.len() < self.block_size as usize {
            return Err(crate::drivers::block::BlockError::BufferTooSmall);
        }
        
        // TODO: Actually issue USB transfer
        // For now, return empty data
        buffer[..self.block_size as usize].fill(0);
        Ok(())
    }
    
    fn write_sector(&self, sector: u64, buffer: &[u8]) -> Result<(), crate::drivers::block::BlockError> {
        if sector >= self.total_blocks {
            return Err(crate::drivers::block::BlockError::InvalidSector);
        }
        if buffer.len() < self.block_size as usize {
            return Err(crate::drivers::block::BlockError::BufferTooSmall);
        }
        
        // TODO: Actually issue USB transfer
        Ok(())
    }
}

lazy_static! {
    /// Registered MSC devices
    static ref MSC_DEVICES: Mutex<Vec<MscDevice>> = Mutex::new(Vec::new());
}

/// Register an MSC device
pub fn register_device(device: MscDevice) {
    crate::println!("    USB Mass Storage: {} {} ({} MB)",
        device.vendor,
        device.product,
        device.capacity_mb());
    MSC_DEVICES.lock().push(device);
}

/// Get all MSC devices
pub fn get_devices() -> Vec<String> {
    MSC_DEVICES.lock()
        .iter()
        .map(|d| alloc::format!("{} {}", d.vendor, d.product))
        .collect()
}

/// Get total MSC device count
pub fn device_count() -> usize {
    MSC_DEVICES.lock().len()
}

/// Initialize MSC subsystem
pub fn init() {
    // Find MSC devices among enumerated USB devices
    for usb_device in super::get_devices() {
        if usb_device.class == UsbClass::MassStorage {
            let mut device = MscDevice::new(usb_device, 0, 0x81, 0x02);
            device.vendor = "USB".to_string();
            device.product = "Storage".to_string();
            device.block_size = 512;
            device.total_blocks = 0; // Unknown until we query
            register_device(device);
        }
    }
}

