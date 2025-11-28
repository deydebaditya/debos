//! Block Device Abstraction
//!
//! Provides a unified interface for block devices.

/// Block device error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockError {
    /// Device not found/initialized
    NoDevice,
    /// Invalid sector number
    InvalidSector,
    /// I/O error
    IoError,
    /// Buffer too small
    BufferTooSmall,
    /// Device is read-only
    ReadOnly,
    /// Request queue full
    QueueFull,
    /// No request buffers available
    NoBuffers,
    /// Operation not supported
    NotSupported,
}

/// Block device trait
pub trait BlockDevice {
    /// Get the number of sectors
    fn sector_count(&self) -> u64;
    
    /// Get the sector size in bytes
    fn sector_size(&self) -> u32;
    
    /// Check if device is read-only
    fn is_read_only(&self) -> bool;
    
    /// Read a sector
    fn read_sector(&self, sector: u64, buf: &mut [u8]) -> Result<(), BlockError>;
    
    /// Write a sector
    fn write_sector(&self, sector: u64, data: &[u8]) -> Result<(), BlockError>;
    
    /// Read multiple sectors
    fn read_sectors(&self, start_sector: u64, count: usize, buf: &mut [u8]) -> Result<(), BlockError> {
        let sector_size = self.sector_size() as usize;
        if buf.len() < count * sector_size {
            return Err(BlockError::BufferTooSmall);
        }
        
        for i in 0..count {
            let offset = i * sector_size;
            self.read_sector(start_sector + i as u64, &mut buf[offset..offset + sector_size])?;
        }
        
        Ok(())
    }
    
    /// Write multiple sectors
    fn write_sectors(&self, start_sector: u64, count: usize, data: &[u8]) -> Result<(), BlockError> {
        if self.is_read_only() {
            return Err(BlockError::ReadOnly);
        }
        
        let sector_size = self.sector_size() as usize;
        if data.len() < count * sector_size {
            return Err(BlockError::BufferTooSmall);
        }
        
        for i in 0..count {
            let offset = i * sector_size;
            self.write_sector(start_sector + i as u64, &data[offset..offset + sector_size])?;
        }
        
        Ok(())
    }
}

/// Read raw bytes at a byte offset
pub fn read_bytes(start_byte: u64, buf: &mut [u8]) -> Result<(), BlockError> {
    use super::virtio::block;
    
    let sector_size = 512u64;
    let start_sector = start_byte / sector_size;
    let offset_in_sector = (start_byte % sector_size) as usize;
    
    let mut sector_buf = [0u8; 512];
    let mut bytes_read = 0;
    let mut current_sector = start_sector;
    
    while bytes_read < buf.len() {
        block::read_sector(current_sector, &mut sector_buf)?;
        
        let start_in_sector = if current_sector == start_sector {
            offset_in_sector
        } else {
            0
        };
        
        let bytes_in_sector = (sector_size as usize - start_in_sector).min(buf.len() - bytes_read);
        
        buf[bytes_read..bytes_read + bytes_in_sector]
            .copy_from_slice(&sector_buf[start_in_sector..start_in_sector + bytes_in_sector]);
        
        bytes_read += bytes_in_sector;
        current_sector += 1;
    }
    
    Ok(())
}

/// Get block device information
pub fn get_device_info() -> Option<(u64, u32, bool)> {
    super::virtio::block::get_info()
}

