//! Keyboard Driver
//!
//! Supports PS/2 keyboard (x86) and VirtIO input (VMs).

use spin::Mutex;
use lazy_static::lazy_static;

use super::event::{InputEvent, KeyCode};
use super::{queue_event, register_device, InputDeviceType};

/// Keyboard state
pub struct Keyboard {
    /// Shift key state
    pub shift: bool,
    
    /// Control key state
    pub ctrl: bool,
    
    /// Alt key state
    pub alt: bool,
    
    /// Caps Lock state
    pub caps_lock: bool,
    
    /// Num Lock state
    pub num_lock: bool,
    
    /// Scroll Lock state
    pub scroll_lock: bool,
}

impl Keyboard {
    pub fn new() -> Self {
        Keyboard {
            shift: false,
            ctrl: false,
            alt: false,
            caps_lock: false,
            num_lock: false,
            scroll_lock: false,
        }
    }
    
    /// Handle a key event
    pub fn handle_key(&mut self, scancode: u16, pressed: bool) {
        // Update modifier state
        match scancode {
            KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT => {
                self.shift = pressed;
            }
            KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL => {
                self.ctrl = pressed;
            }
            KeyCode::KEY_LEFTALT | KeyCode::KEY_RIGHTALT => {
                self.alt = pressed;
            }
            KeyCode::KEY_CAPSLOCK if pressed => {
                self.caps_lock = !self.caps_lock;
            }
            KeyCode::KEY_NUMLOCK if pressed => {
                self.num_lock = !self.num_lock;
            }
            KeyCode::KEY_SCROLLLOCK if pressed => {
                self.scroll_lock = !self.scroll_lock;
            }
            _ => {}
        }
        
        // Queue the event
        let event = if pressed {
            InputEvent::key_press(scancode)
        } else {
            InputEvent::key_release(scancode)
        };
        
        queue_event(event);
    }
    
    /// Check if shift is effective (shift XOR caps_lock for letters)
    pub fn is_shifted(&self) -> bool {
        self.shift ^ self.caps_lock
    }
}

lazy_static! {
    /// Global keyboard state
    pub static ref KEYBOARD: Mutex<Keyboard> = Mutex::new(Keyboard::new());
}

/// Keyboard layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardLayout {
    UsQwerty,
    UkQwerty,
    Dvorak,
}

impl Default for KeyboardLayout {
    fn default() -> Self {
        KeyboardLayout::UsQwerty
    }
}

/// Initialize keyboard driver
pub fn init() {
    // Register keyboard device
    register_device("keyboard0", InputDeviceType::Keyboard);
    
    // Platform-specific initialization
    #[cfg(target_arch = "x86_64")]
    init_ps2_keyboard();
    
    #[cfg(target_arch = "aarch64")]
    init_virtio_keyboard();
}

#[cfg(target_arch = "x86_64")]
fn init_ps2_keyboard() {
    // PS/2 keyboard initialization would go here
    // For now, we rely on serial input
}

#[cfg(target_arch = "aarch64")]
fn init_virtio_keyboard() {
    // VirtIO keyboard initialization would go here
    // For now, we rely on UART input
}

/// Handle keyboard interrupt (called from ISR)
pub fn handle_interrupt(scancode: u8) {
    let mut kbd = KEYBOARD.lock();
    
    // Check for key release (bit 7 set)
    let pressed = (scancode & 0x80) == 0;
    let code = (scancode & 0x7F) as u16;
    
    // Convert PS/2 scancode set 1 to our key codes
    let key = ps2_to_keycode(code);
    
    if key != KeyCode::KEY_RESERVED {
        kbd.handle_key(key, pressed);
    }
}

/// Convert PS/2 scancode set 1 to our key codes
fn ps2_to_keycode(scancode: u16) -> u16 {
    match scancode {
        0x01 => KeyCode::KEY_ESC,
        0x02 => KeyCode::KEY_1,
        0x03 => KeyCode::KEY_2,
        0x04 => KeyCode::KEY_3,
        0x05 => KeyCode::KEY_4,
        0x06 => KeyCode::KEY_5,
        0x07 => KeyCode::KEY_6,
        0x08 => KeyCode::KEY_7,
        0x09 => KeyCode::KEY_8,
        0x0A => KeyCode::KEY_9,
        0x0B => KeyCode::KEY_0,
        0x0C => KeyCode::KEY_MINUS,
        0x0D => KeyCode::KEY_EQUAL,
        0x0E => KeyCode::KEY_BACKSPACE,
        0x0F => KeyCode::KEY_TAB,
        0x10 => KeyCode::KEY_Q,
        0x11 => KeyCode::KEY_W,
        0x12 => KeyCode::KEY_E,
        0x13 => KeyCode::KEY_R,
        0x14 => KeyCode::KEY_T,
        0x15 => KeyCode::KEY_Y,
        0x16 => KeyCode::KEY_U,
        0x17 => KeyCode::KEY_I,
        0x18 => KeyCode::KEY_O,
        0x19 => KeyCode::KEY_P,
        0x1A => KeyCode::KEY_LEFTBRACE,
        0x1B => KeyCode::KEY_RIGHTBRACE,
        0x1C => KeyCode::KEY_ENTER,
        0x1D => KeyCode::KEY_LEFTCTRL,
        0x1E => KeyCode::KEY_A,
        0x1F => KeyCode::KEY_S,
        0x20 => KeyCode::KEY_D,
        0x21 => KeyCode::KEY_F,
        0x22 => KeyCode::KEY_G,
        0x23 => KeyCode::KEY_H,
        0x24 => KeyCode::KEY_J,
        0x25 => KeyCode::KEY_K,
        0x26 => KeyCode::KEY_L,
        0x27 => KeyCode::KEY_SEMICOLON,
        0x28 => KeyCode::KEY_APOSTROPHE,
        0x29 => KeyCode::KEY_GRAVE,
        0x2A => KeyCode::KEY_LEFTSHIFT,
        0x2B => KeyCode::KEY_BACKSLASH,
        0x2C => KeyCode::KEY_Z,
        0x2D => KeyCode::KEY_X,
        0x2E => KeyCode::KEY_C,
        0x2F => KeyCode::KEY_V,
        0x30 => KeyCode::KEY_B,
        0x31 => KeyCode::KEY_N,
        0x32 => KeyCode::KEY_M,
        0x33 => KeyCode::KEY_COMMA,
        0x34 => KeyCode::KEY_DOT,
        0x35 => KeyCode::KEY_SLASH,
        0x36 => KeyCode::KEY_RIGHTSHIFT,
        0x38 => KeyCode::KEY_LEFTALT,
        0x39 => KeyCode::KEY_SPACE,
        0x3A => KeyCode::KEY_CAPSLOCK,
        0x3B => KeyCode::KEY_F1,
        0x3C => KeyCode::KEY_F2,
        0x3D => KeyCode::KEY_F3,
        0x3E => KeyCode::KEY_F4,
        0x3F => KeyCode::KEY_F5,
        0x40 => KeyCode::KEY_F6,
        0x41 => KeyCode::KEY_F7,
        0x42 => KeyCode::KEY_F8,
        0x43 => KeyCode::KEY_F9,
        0x44 => KeyCode::KEY_F10,
        0x45 => KeyCode::KEY_NUMLOCK,
        0x46 => KeyCode::KEY_SCROLLLOCK,
        0x57 => KeyCode::KEY_F11,
        0x58 => KeyCode::KEY_F12,
        _ => KeyCode::KEY_RESERVED,
    }
}

/// Read a character from keyboard (blocking)
pub fn read_char() -> Option<char> {
    if let Some(event) = super::poll_event() {
        if event.event_type == super::event::InputEventType::Key && event.value == 1 {
            let kbd = KEYBOARD.lock();
            return super::event::scancode_to_ascii(event.code, kbd.is_shifted());
        }
    }
    None
}

