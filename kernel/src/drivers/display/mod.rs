//! Display Subsystem
//!
//! Provides framebuffer and text console support.

pub mod console;

use spin::Mutex;
use lazy_static::lazy_static;

/// Display information
#[derive(Debug, Clone, Copy)]
pub struct DisplayInfo {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Bits per pixel
    pub bpp: u8,
    /// Pitch (bytes per row)
    pub pitch: u32,
}

lazy_static! {
    /// Current display info
    static ref DISPLAY_INFO: Mutex<Option<DisplayInfo>> = Mutex::new(None);
}

/// Initialize display subsystem
pub fn init() {
    crate::println!("  [..] Initializing display subsystem...");
    
    // Try to get display from VirtIO-GPU
    if let Some((width, height)) = crate::drivers::virtio::gpu::dimensions() {
        let info = DisplayInfo {
            width,
            height,
            bpp: 32,
            pitch: width * 4,
        };
        *DISPLAY_INFO.lock() = Some(info);
        
        // Initialize console
        console::init(width, height);
        
        crate::println!("  Display: {}x{}x{}", width, height, 32);
    } else {
        crate::println!("  No display available (headless mode)");
    }
    
    crate::println!("  [OK] Display subsystem initialized");
}

/// Get display info
pub fn get_info() -> Option<DisplayInfo> {
    *DISPLAY_INFO.lock()
}

/// Check if display is available
pub fn is_available() -> bool {
    DISPLAY_INFO.lock().is_some()
}

