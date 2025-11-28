//! Input Subsystem
//!
//! Provides keyboard, mouse, and other input device support.

pub mod event;
pub mod keyboard;
pub mod mouse;

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

pub use event::{InputEvent, InputEventType, KeyCode, MouseButton};
pub use keyboard::{Keyboard, KeyboardLayout};
pub use mouse::Mouse;

/// Maximum events in queue
const MAX_EVENTS: usize = 256;

lazy_static! {
    /// Global input event queue
    static ref INPUT_QUEUE: Mutex<VecDeque<InputEvent>> = Mutex::new(VecDeque::new());
    
    /// Registered input devices
    static ref INPUT_DEVICES: Mutex<Vec<InputDeviceInfo>> = Mutex::new(Vec::new());
}

/// Input device info
#[derive(Debug, Clone)]
pub struct InputDeviceInfo {
    pub id: u32,
    pub name: &'static str,
    pub device_type: InputDeviceType,
}

/// Input device types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDeviceType {
    Keyboard,
    Mouse,
    Touchpad,
    Touchscreen,
    Gamepad,
    Other,
}

/// Initialize the input subsystem
pub fn init() {
    crate::println!("  [..] Initializing input subsystem...");
    
    // Initialize keyboard
    keyboard::init();
    
    // Initialize mouse
    mouse::init();
    
    crate::println!("  [OK] Input subsystem initialized");
}

/// Queue an input event
pub fn queue_event(event: InputEvent) {
    let mut queue = INPUT_QUEUE.lock();
    
    if queue.len() >= MAX_EVENTS {
        queue.pop_front();  // Drop oldest event
    }
    
    queue.push_back(event);
}

/// Poll for the next input event
pub fn poll_event() -> Option<InputEvent> {
    INPUT_QUEUE.lock().pop_front()
}

/// Check if there are pending events
pub fn has_events() -> bool {
    !INPUT_QUEUE.lock().is_empty()
}

/// Get number of pending events
pub fn pending_count() -> usize {
    INPUT_QUEUE.lock().len()
}

/// Register an input device
pub fn register_device(name: &'static str, device_type: InputDeviceType) -> u32 {
    let mut devices = INPUT_DEVICES.lock();
    let id = devices.len() as u32;
    devices.push(InputDeviceInfo {
        id,
        name,
        device_type,
    });
    id
}

/// Get list of input devices
pub fn list_devices() -> Vec<InputDeviceInfo> {
    INPUT_DEVICES.lock().clone()
}

