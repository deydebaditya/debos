//! VirtIO Input Driver
//!
//! Implements VirtIO-Input device for keyboard and mouse support.

use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

use super::mmio::MmioDevice;
use super::queue::VirtQueue;
use crate::drivers::input::{InputEvent, InputEventType, queue_event};

/// VirtIO input device type
pub const VIRTIO_INPUT_DEVICE_ID: u32 = 18;

/// VirtIO input event
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VirtioInputEvent {
    pub event_type: u16,
    pub code: u16,
    pub value: u32,
}

impl VirtioInputEvent {
    pub fn to_input_event(&self) -> InputEvent {
        let event_type = match self.event_type {
            0x00 => InputEventType::Sync,
            0x01 => InputEventType::Key,
            0x02 => InputEventType::Relative,
            0x03 => InputEventType::Absolute,
            _ => InputEventType::Misc,
        };
        
        InputEvent {
            timestamp: crate::scheduler::ticks(),
            event_type,
            code: self.code,
            value: self.value as i32,
        }
    }
}

/// VirtIO input device
pub struct VirtioInput {
    /// MMIO device
    mmio: MmioDevice,
    
    /// Event queue (device -> driver)
    event_queue: VirtQueue,
    
    /// Status queue (driver -> device)
    status_queue: VirtQueue,
    
    /// Event buffers
    event_buffers: Vec<VirtioInputEvent>,
    
    /// Is device ready
    ready: bool,
    
    /// Device name
    name: &'static str,
}

lazy_static! {
    /// Global VirtIO input devices
    pub static ref VIRTIO_INPUTS: Mutex<Vec<VirtioInput>> = Mutex::new(Vec::new());
}

impl VirtioInput {
    /// Create a new VirtIO input device
    pub fn new(base_addr: usize, name: &'static str) -> Result<Self, &'static str> {
        let mmio = MmioDevice::probe(base_addr)?;
        
        // Check device type
        let device_id = mmio.device_id();
        if device_id != VIRTIO_INPUT_DEVICE_ID {
            return Err("Not a VirtIO input device");
        }
        
        // Create queues
        let event_queue = VirtQueue::new(0, 64);
        let status_queue = VirtQueue::new(1, 64);
        
        Ok(VirtioInput {
            mmio,
            event_queue,
            status_queue,
            event_buffers: Vec::new(),
            ready: false,
            name,
        })
    }
    
    /// Initialize the device
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Reset device
        self.mmio.reset();
        
        // Acknowledge device
        self.mmio.set_status(0x01); // ACKNOWLEDGE
        self.mmio.set_status(0x03); // DRIVER
        
        // Negotiate features (none for input)
        self.mmio.write_features(0);
        
        // Features OK
        self.mmio.set_status(0x0B); // FEATURES_OK
        
        // Initialize queues
        self.mmio.select_queue(0);
        self.mmio.set_queue_size(64);
        self.mmio.set_queue_desc(self.event_queue.desc_addr());
        self.mmio.set_queue_avail(self.event_queue.avail_addr());
        self.mmio.set_queue_used(self.event_queue.used_addr());
        self.mmio.enable_queue();
        
        self.mmio.select_queue(1);
        self.mmio.set_queue_size(64);
        self.mmio.set_queue_desc(self.status_queue.desc_addr());
        self.mmio.set_queue_avail(self.status_queue.avail_addr());
        self.mmio.set_queue_used(self.status_queue.used_addr());
        self.mmio.enable_queue();
        
        // Set up event buffers
        self.setup_event_buffers()?;
        
        // Driver ready
        self.mmio.set_status(0x0F); // DRIVER_OK
        
        self.ready = true;
        Ok(())
    }
    
    /// Set up event buffers
    fn setup_event_buffers(&mut self) -> Result<(), &'static str> {
        for _ in 0..16 {
            let event = VirtioInputEvent::default();
            self.event_buffers.push(event);
        }
        
        for event in &self.event_buffers {
            let addr = event as *const _ as u64;
            self.event_queue.add_buffer(&[addr], &[], core::mem::size_of::<VirtioInputEvent>() as u32)?;
        }
        
        // Notify device about available buffers
        self.mmio.notify(0);
        
        Ok(())
    }
    
    /// Poll for events
    pub fn poll(&mut self) -> Option<InputEvent> {
        if !self.ready {
            return None;
        }
        
        // Check for completed event buffers
        if let Some((idx, _len)) = self.event_queue.pop_used() {
            if idx < self.event_buffers.len() {
                let event = self.event_buffers[idx].to_input_event();
                
                // Re-add buffer to queue
                let addr = &self.event_buffers[idx] as *const _ as u64;
                let _ = self.event_queue.add_buffer(&[addr], &[], core::mem::size_of::<VirtioInputEvent>() as u32);
                self.mmio.notify(0);
                
                return Some(event);
            }
        }
        
        None
    }
    
    /// Get device name
    pub fn name(&self) -> &'static str {
        self.name
    }
}

/// Initialize VirtIO input devices
pub fn init() -> Result<(), &'static str> {
    // Scan for VirtIO input devices at known MMIO addresses
    let mmio_addrs: &[(usize, &'static str)] = &[
        (0x0A003200, "virtio-keyboard"),
        (0x0A003000, "virtio-mouse"),
    ];
    
    let mut devices = VIRTIO_INPUTS.lock();
    
    for &(addr, name) in mmio_addrs {
        if let Ok(mut input) = VirtioInput::new(addr, name) {
            if input.init().is_ok() {
                crate::println!("    VirtIO-Input: {} at {:#x}", name, addr);
                devices.push(input);
            }
        }
    }
    
    Ok(())
}

/// Poll all VirtIO input devices
pub fn poll_all() {
    let mut devices = VIRTIO_INPUTS.lock();
    
    for device in devices.iter_mut() {
        while let Some(event) = device.poll() {
            queue_event(event);
        }
    }
}

/// Check if any VirtIO input devices are available
pub fn is_available() -> bool {
    !VIRTIO_INPUTS.lock().is_empty()
}

