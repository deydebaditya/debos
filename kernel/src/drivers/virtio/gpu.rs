//! VirtIO GPU Driver
//!
//! Implements VirtIO-GPU device for framebuffer and 2D graphics.

use alloc::vec::Vec;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;

use super::mmio::MmioDevice;
use super::queue::VirtQueue;

/// VirtIO GPU device type
pub const VIRTIO_GPU_DEVICE_ID: u32 = 16;

/// VirtIO GPU command types
pub mod Commands {
    pub const GET_DISPLAY_INFO: u32 = 0x0100;
    pub const RESOURCE_CREATE_2D: u32 = 0x0101;
    pub const RESOURCE_UNREF: u32 = 0x0102;
    pub const SET_SCANOUT: u32 = 0x0103;
    pub const RESOURCE_FLUSH: u32 = 0x0104;
    pub const TRANSFER_TO_HOST_2D: u32 = 0x0105;
    pub const RESOURCE_ATTACH_BACKING: u32 = 0x0106;
    pub const RESOURCE_DETACH_BACKING: u32 = 0x0107;
    pub const GET_CAPSET_INFO: u32 = 0x0108;
    pub const GET_CAPSET: u32 = 0x0109;
}

/// VirtIO GPU response types
pub mod Responses {
    pub const OK_NODATA: u32 = 0x1100;
    pub const OK_DISPLAY_INFO: u32 = 0x1101;
    pub const OK_CAPSET_INFO: u32 = 0x1102;
    pub const OK_CAPSET: u32 = 0x1103;
    pub const ERR_UNSPEC: u32 = 0x1200;
    pub const ERR_OUT_OF_MEMORY: u32 = 0x1201;
    pub const ERR_INVALID_SCANOUT_ID: u32 = 0x1202;
    pub const ERR_INVALID_RESOURCE_ID: u32 = 0x1203;
    pub const ERR_INVALID_CONTEXT_ID: u32 = 0x1204;
    pub const ERR_INVALID_PARAMETER: u32 = 0x1205;
}

/// Pixel formats
pub mod PixelFormat {
    pub const B8G8R8A8_UNORM: u32 = 1;
    pub const B8G8R8X8_UNORM: u32 = 2;
    pub const A8R8G8B8_UNORM: u32 = 3;
    pub const X8R8G8B8_UNORM: u32 = 4;
    pub const R8G8B8A8_UNORM: u32 = 67;
    pub const X8B8G8R8_UNORM: u32 = 68;
    pub const A8B8G8R8_UNORM: u32 = 121;
    pub const R8G8B8X8_UNORM: u32 = 134;
}

/// VirtIO GPU control header
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CtrlHeader {
    pub hdr_type: u32,
    pub flags: u32,
    pub fence_id: u64,
    pub ctx_id: u32,
    pub padding: u32,
}

/// Display info
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DisplayInfo {
    pub rect: Rect,
    pub enabled: u32,
    pub flags: u32,
}

/// Rectangle
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Framebuffer info
#[derive(Clone)]
pub struct FramebufferInfo {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u8,
    pub buffer: Vec<u8>,
}

/// VirtIO GPU device
pub struct VirtioGpu {
    /// MMIO device
    mmio: MmioDevice,
    
    /// Control queue
    ctrl_queue: VirtQueue,
    
    /// Cursor queue
    cursor_queue: VirtQueue,
    
    /// Framebuffer
    framebuffer: Option<FramebufferInfo>,
    
    /// Display width
    width: u32,
    
    /// Display height
    height: u32,
    
    /// Is device ready
    ready: bool,
    
    /// Resource ID counter
    next_resource_id: u32,
}

lazy_static! {
    /// Global VirtIO GPU device
    pub static ref VIRTIO_GPU: Mutex<Option<VirtioGpu>> = Mutex::new(None);
}

impl VirtioGpu {
    /// Create a new VirtIO GPU device
    pub fn new(base_addr: usize) -> Result<Self, &'static str> {
        let mmio = MmioDevice::probe(base_addr)?;
        
        // Check device type
        let device_id = mmio.device_id();
        if device_id != VIRTIO_GPU_DEVICE_ID {
            return Err("Not a VirtIO GPU device");
        }
        
        // Create queues
        let ctrl_queue = VirtQueue::new(0, 64);
        let cursor_queue = VirtQueue::new(1, 16);
        
        Ok(VirtioGpu {
            mmio,
            ctrl_queue,
            cursor_queue,
            framebuffer: None,
            width: 0,
            height: 0,
            ready: false,
            next_resource_id: 1,
        })
    }
    
    /// Initialize the device
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Reset device
        self.mmio.reset();
        
        // Acknowledge device
        self.mmio.set_status(0x01); // ACKNOWLEDGE
        self.mmio.set_status(0x03); // DRIVER
        
        // Negotiate features
        self.mmio.write_features(0);
        
        // Features OK
        self.mmio.set_status(0x0B); // FEATURES_OK
        
        // Initialize control queue
        self.mmio.select_queue(0);
        self.mmio.set_queue_size(64);
        self.mmio.set_queue_desc(self.ctrl_queue.desc_addr());
        self.mmio.set_queue_avail(self.ctrl_queue.avail_addr());
        self.mmio.set_queue_used(self.ctrl_queue.used_addr());
        self.mmio.enable_queue();
        
        // Initialize cursor queue
        self.mmio.select_queue(1);
        self.mmio.set_queue_size(16);
        self.mmio.set_queue_desc(self.cursor_queue.desc_addr());
        self.mmio.set_queue_avail(self.cursor_queue.avail_addr());
        self.mmio.set_queue_used(self.cursor_queue.used_addr());
        self.mmio.enable_queue();
        
        // Driver ready
        self.mmio.set_status(0x0F); // DRIVER_OK
        
        // Get display info
        self.get_display_info()?;
        
        self.ready = true;
        Ok(())
    }
    
    /// Get display info from device
    fn get_display_info(&mut self) -> Result<(), &'static str> {
        // For now, use default resolution
        // TODO: Actually query the device
        self.width = 1024;
        self.height = 768;
        Ok(())
    }
    
    /// Create framebuffer
    pub fn create_framebuffer(&mut self) -> Result<(), &'static str> {
        if !self.ready {
            return Err("Device not ready");
        }
        
        let width = self.width;
        let height = self.height;
        let bpp = 4; // 32-bit color
        let pitch = width * bpp;
        let size = (pitch * height) as usize;
        
        let buffer = alloc::vec![0u8; size];
        
        self.framebuffer = Some(FramebufferInfo {
            width,
            height,
            pitch,
            bpp: 32,
            buffer,
        });
        
        Ok(())
    }
    
    /// Get framebuffer info
    pub fn framebuffer(&self) -> Option<&FramebufferInfo> {
        self.framebuffer.as_ref()
    }
    
    /// Get mutable framebuffer
    pub fn framebuffer_mut(&mut self) -> Option<&mut FramebufferInfo> {
        self.framebuffer.as_mut()
    }
    
    /// Set a pixel
    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        if let Some(fb) = &mut self.framebuffer {
            if x < fb.width && y < fb.height {
                let offset = ((y * fb.pitch) + (x * 4)) as usize;
                if offset + 3 < fb.buffer.len() {
                    // BGRA format
                    fb.buffer[offset] = b;
                    fb.buffer[offset + 1] = g;
                    fb.buffer[offset + 2] = r;
                    fb.buffer[offset + 3] = 255; // Alpha
                }
            }
        }
    }
    
    /// Fill rectangle
    pub fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, r: u8, g: u8, b: u8) {
        for py in y..(y + height) {
            for px in x..(x + width) {
                self.set_pixel(px, py, r, g, b);
            }
        }
    }
    
    /// Clear screen
    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        if let Some(fb) = &mut self.framebuffer {
            let width = fb.width;
            let height = fb.height;
            self.fill_rect(0, 0, width, height, r, g, b);
        }
    }
    
    /// Flush framebuffer to display
    pub fn flush(&mut self) -> Result<(), &'static str> {
        if !self.ready {
            return Err("Device not ready");
        }
        
        // TODO: Send RESOURCE_FLUSH command to device
        // For now, the framebuffer is maintained in memory
        
        Ok(())
    }
    
    /// Get display dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Initialize VirtIO GPU driver
pub fn init() -> Result<(), &'static str> {
    // Scan for VirtIO GPU device at known MMIO addresses
    let mmio_addrs: &[usize] = &[
        0x0A003400,  // QEMU virt machine VirtIO MMIO
        0x0A003600,
    ];
    
    for &addr in mmio_addrs {
        if let Ok(mut gpu) = VirtioGpu::new(addr) {
            if gpu.init().is_ok() {
                if gpu.create_framebuffer().is_ok() {
                    let (w, h) = gpu.dimensions();
                    crate::println!("    VirtIO-GPU: {}x{} at {:#x}", w, h, addr);
                    *VIRTIO_GPU.lock() = Some(gpu);
                    return Ok(());
                }
            }
        }
    }
    
    // No VirtIO GPU device found (this is OK for headless mode)
    Ok(())
}

/// Check if VirtIO GPU is available
pub fn is_available() -> bool {
    VIRTIO_GPU.lock().is_some()
}

/// Get display dimensions
pub fn dimensions() -> Option<(u32, u32)> {
    VIRTIO_GPU.lock().as_ref().map(|g| g.dimensions())
}

/// Set a pixel
pub fn set_pixel(x: u32, y: u32, r: u8, g: u8, b: u8) {
    if let Some(gpu) = VIRTIO_GPU.lock().as_mut() {
        gpu.set_pixel(x, y, r, g, b);
    }
}

/// Fill rectangle
pub fn fill_rect(x: u32, y: u32, width: u32, height: u32, r: u8, g: u8, b: u8) {
    if let Some(gpu) = VIRTIO_GPU.lock().as_mut() {
        gpu.fill_rect(x, y, width, height, r, g, b);
    }
}

/// Clear screen
pub fn clear(r: u8, g: u8, b: u8) {
    if let Some(gpu) = VIRTIO_GPU.lock().as_mut() {
        gpu.clear(r, g, b);
    }
}

/// Flush display
pub fn flush() {
    if let Some(gpu) = VIRTIO_GPU.lock().as_mut() {
        let _ = gpu.flush();
    }
}

