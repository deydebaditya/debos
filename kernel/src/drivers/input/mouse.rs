//! Mouse Driver
//!
//! Supports PS/2 mouse (x86) and VirtIO input (VMs).

use spin::Mutex;
use lazy_static::lazy_static;

use super::event::{InputEvent, MouseButton, RelAxis};
use super::{queue_event, register_device, InputDeviceType};

/// Mouse state
pub struct Mouse {
    /// Current X position (relative tracking)
    pub x: i32,
    
    /// Current Y position (relative tracking)
    pub y: i32,
    
    /// Left button state
    pub left_button: bool,
    
    /// Right button state
    pub right_button: bool,
    
    /// Middle button state
    pub middle_button: bool,
    
    /// Scroll wheel position
    pub scroll: i32,
}

impl Mouse {
    pub fn new() -> Self {
        Mouse {
            x: 0,
            y: 0,
            left_button: false,
            right_button: false,
            middle_button: false,
            scroll: 0,
        }
    }
    
    /// Handle mouse movement
    pub fn handle_motion(&mut self, dx: i32, dy: i32) {
        self.x = self.x.saturating_add(dx);
        self.y = self.y.saturating_add(dy);
        
        if dx != 0 {
            queue_event(InputEvent::rel_motion(RelAxis::X, dx));
        }
        if dy != 0 {
            queue_event(InputEvent::rel_motion(RelAxis::Y, dy));
        }
    }
    
    /// Handle mouse button
    pub fn handle_button(&mut self, button: MouseButton, pressed: bool) {
        match button {
            MouseButton::Left => self.left_button = pressed,
            MouseButton::Right => self.right_button = pressed,
            MouseButton::Middle => self.middle_button = pressed,
            _ => {}
        }
        
        let event = if pressed {
            InputEvent::mouse_button_press(button)
        } else {
            InputEvent::mouse_button_release(button)
        };
        queue_event(event);
    }
    
    /// Handle scroll wheel
    pub fn handle_scroll(&mut self, delta: i32) {
        self.scroll = self.scroll.saturating_add(delta);
        queue_event(InputEvent::rel_motion(RelAxis::Wheel, delta));
    }
}

lazy_static! {
    /// Global mouse state
    pub static ref MOUSE: Mutex<Mouse> = Mutex::new(Mouse::new());
}

/// Initialize mouse driver
pub fn init() {
    // Register mouse device
    register_device("mouse0", InputDeviceType::Mouse);
    
    // Platform-specific initialization
    #[cfg(target_arch = "x86_64")]
    init_ps2_mouse();
    
    #[cfg(target_arch = "aarch64")]
    init_virtio_mouse();
}

#[cfg(target_arch = "x86_64")]
fn init_ps2_mouse() {
    // PS/2 mouse initialization would go here
    // For QEMU, we typically use USB or VirtIO mouse
}

#[cfg(target_arch = "aarch64")]
fn init_virtio_mouse() {
    // VirtIO mouse initialization would go here
}

/// Handle mouse interrupt (called from ISR)
pub fn handle_interrupt(packet: &[u8]) {
    if packet.len() < 3 {
        return;
    }
    
    let mut mouse = MOUSE.lock();
    
    // Parse PS/2 mouse packet
    let buttons = packet[0];
    let dx = packet[1] as i8 as i32;
    let dy = -(packet[2] as i8 as i32); // Y is inverted
    
    // Handle buttons
    let left = (buttons & 0x01) != 0;
    let right = (buttons & 0x02) != 0;
    let middle = (buttons & 0x04) != 0;
    
    if left != mouse.left_button {
        mouse.handle_button(MouseButton::Left, left);
    }
    if right != mouse.right_button {
        mouse.handle_button(MouseButton::Right, right);
    }
    if middle != mouse.middle_button {
        mouse.handle_button(MouseButton::Middle, middle);
    }
    
    // Handle motion
    if dx != 0 || dy != 0 {
        mouse.handle_motion(dx, dy);
    }
    
    // Handle scroll wheel (if present in extended packet)
    if packet.len() >= 4 {
        let scroll = packet[3] as i8 as i32;
        if scroll != 0 {
            mouse.handle_scroll(scroll);
        }
    }
}

/// Get current mouse position
pub fn get_position() -> (i32, i32) {
    let mouse = MOUSE.lock();
    (mouse.x, mouse.y)
}

/// Get button states
pub fn get_buttons() -> (bool, bool, bool) {
    let mouse = MOUSE.lock();
    (mouse.left_button, mouse.right_button, mouse.middle_button)
}

