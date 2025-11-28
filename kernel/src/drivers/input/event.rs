//! Input Events
//!
//! Event types for keyboard, mouse, and other input devices.

/// Input event (similar to Linux evdev)
#[derive(Debug, Clone, Copy)]
pub struct InputEvent {
    /// Timestamp (ticks since boot)
    pub timestamp: u64,
    
    /// Event type
    pub event_type: InputEventType,
    
    /// Event code (key code, axis, button)
    pub code: u16,
    
    /// Event value (1 = pressed, 0 = released, or axis value)
    pub value: i32,
}

impl InputEvent {
    /// Create a key press event
    pub fn key_press(code: u16) -> Self {
        InputEvent {
            timestamp: crate::scheduler::ticks(),
            event_type: InputEventType::Key,
            code,
            value: 1,
        }
    }
    
    /// Create a key release event
    pub fn key_release(code: u16) -> Self {
        InputEvent {
            timestamp: crate::scheduler::ticks(),
            event_type: InputEventType::Key,
            code,
            value: 0,
        }
    }
    
    /// Create a mouse button press
    pub fn mouse_button_press(button: MouseButton) -> Self {
        InputEvent {
            timestamp: crate::scheduler::ticks(),
            event_type: InputEventType::Key,
            code: button as u16,
            value: 1,
        }
    }
    
    /// Create a mouse button release
    pub fn mouse_button_release(button: MouseButton) -> Self {
        InputEvent {
            timestamp: crate::scheduler::ticks(),
            event_type: InputEventType::Key,
            code: button as u16,
            value: 0,
        }
    }
    
    /// Create a relative motion event (mouse movement)
    pub fn rel_motion(axis: RelAxis, delta: i32) -> Self {
        InputEvent {
            timestamp: crate::scheduler::ticks(),
            event_type: InputEventType::Relative,
            code: axis as u16,
            value: delta,
        }
    }
    
    /// Create a sync event
    pub fn sync() -> Self {
        InputEvent {
            timestamp: crate::scheduler::ticks(),
            event_type: InputEventType::Sync,
            code: 0,
            value: 0,
        }
    }
}

/// Input event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum InputEventType {
    /// Synchronization event (end of event batch)
    Sync = 0x00,
    
    /// Key/button event
    Key = 0x01,
    
    /// Relative movement (mouse)
    Relative = 0x02,
    
    /// Absolute positioning (touchscreen)
    Absolute = 0x03,
    
    /// Miscellaneous events
    Misc = 0x04,
    
    /// LED state
    Led = 0x11,
}

/// Relative axis codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum RelAxis {
    X = 0x00,
    Y = 0x01,
    Z = 0x02,
    Wheel = 0x08,
    HWheel = 0x06,
}

/// Mouse buttons
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MouseButton {
    Left = 0x110,
    Right = 0x111,
    Middle = 0x112,
    Side = 0x113,
    Extra = 0x114,
}

/// Key codes (USB HID compatible)
#[allow(dead_code)]
pub mod KeyCode {
    pub const KEY_RESERVED: u16 = 0;
    pub const KEY_ESC: u16 = 1;
    pub const KEY_1: u16 = 2;
    pub const KEY_2: u16 = 3;
    pub const KEY_3: u16 = 4;
    pub const KEY_4: u16 = 5;
    pub const KEY_5: u16 = 6;
    pub const KEY_6: u16 = 7;
    pub const KEY_7: u16 = 8;
    pub const KEY_8: u16 = 9;
    pub const KEY_9: u16 = 10;
    pub const KEY_0: u16 = 11;
    pub const KEY_MINUS: u16 = 12;
    pub const KEY_EQUAL: u16 = 13;
    pub const KEY_BACKSPACE: u16 = 14;
    pub const KEY_TAB: u16 = 15;
    pub const KEY_Q: u16 = 16;
    pub const KEY_W: u16 = 17;
    pub const KEY_E: u16 = 18;
    pub const KEY_R: u16 = 19;
    pub const KEY_T: u16 = 20;
    pub const KEY_Y: u16 = 21;
    pub const KEY_U: u16 = 22;
    pub const KEY_I: u16 = 23;
    pub const KEY_O: u16 = 24;
    pub const KEY_P: u16 = 25;
    pub const KEY_LEFTBRACE: u16 = 26;
    pub const KEY_RIGHTBRACE: u16 = 27;
    pub const KEY_ENTER: u16 = 28;
    pub const KEY_LEFTCTRL: u16 = 29;
    pub const KEY_A: u16 = 30;
    pub const KEY_S: u16 = 31;
    pub const KEY_D: u16 = 32;
    pub const KEY_F: u16 = 33;
    pub const KEY_G: u16 = 34;
    pub const KEY_H: u16 = 35;
    pub const KEY_J: u16 = 36;
    pub const KEY_K: u16 = 37;
    pub const KEY_L: u16 = 38;
    pub const KEY_SEMICOLON: u16 = 39;
    pub const KEY_APOSTROPHE: u16 = 40;
    pub const KEY_GRAVE: u16 = 41;
    pub const KEY_LEFTSHIFT: u16 = 42;
    pub const KEY_BACKSLASH: u16 = 43;
    pub const KEY_Z: u16 = 44;
    pub const KEY_X: u16 = 45;
    pub const KEY_C: u16 = 46;
    pub const KEY_V: u16 = 47;
    pub const KEY_B: u16 = 48;
    pub const KEY_N: u16 = 49;
    pub const KEY_M: u16 = 50;
    pub const KEY_COMMA: u16 = 51;
    pub const KEY_DOT: u16 = 52;
    pub const KEY_SLASH: u16 = 53;
    pub const KEY_RIGHTSHIFT: u16 = 54;
    pub const KEY_KPASTERISK: u16 = 55;
    pub const KEY_LEFTALT: u16 = 56;
    pub const KEY_SPACE: u16 = 57;
    pub const KEY_CAPSLOCK: u16 = 58;
    pub const KEY_F1: u16 = 59;
    pub const KEY_F2: u16 = 60;
    pub const KEY_F3: u16 = 61;
    pub const KEY_F4: u16 = 62;
    pub const KEY_F5: u16 = 63;
    pub const KEY_F6: u16 = 64;
    pub const KEY_F7: u16 = 65;
    pub const KEY_F8: u16 = 66;
    pub const KEY_F9: u16 = 67;
    pub const KEY_F10: u16 = 68;
    pub const KEY_NUMLOCK: u16 = 69;
    pub const KEY_SCROLLLOCK: u16 = 70;
    pub const KEY_F11: u16 = 87;
    pub const KEY_F12: u16 = 88;
    pub const KEY_HOME: u16 = 102;
    pub const KEY_UP: u16 = 103;
    pub const KEY_PAGEUP: u16 = 104;
    pub const KEY_LEFT: u16 = 105;
    pub const KEY_RIGHT: u16 = 106;
    pub const KEY_END: u16 = 107;
    pub const KEY_DOWN: u16 = 108;
    pub const KEY_PAGEDOWN: u16 = 109;
    pub const KEY_INSERT: u16 = 110;
    pub const KEY_DELETE: u16 = 111;
    pub const KEY_RIGHTCTRL: u16 = 97;
    pub const KEY_RIGHTALT: u16 = 100;
}

/// Convert scancode to ASCII (simple US layout)
pub fn scancode_to_ascii(scancode: u16, shift: bool) -> Option<char> {
    let lower = match scancode {
        KeyCode::KEY_A => 'a',
        KeyCode::KEY_B => 'b',
        KeyCode::KEY_C => 'c',
        KeyCode::KEY_D => 'd',
        KeyCode::KEY_E => 'e',
        KeyCode::KEY_F => 'f',
        KeyCode::KEY_G => 'g',
        KeyCode::KEY_H => 'h',
        KeyCode::KEY_I => 'i',
        KeyCode::KEY_J => 'j',
        KeyCode::KEY_K => 'k',
        KeyCode::KEY_L => 'l',
        KeyCode::KEY_M => 'm',
        KeyCode::KEY_N => 'n',
        KeyCode::KEY_O => 'o',
        KeyCode::KEY_P => 'p',
        KeyCode::KEY_Q => 'q',
        KeyCode::KEY_R => 'r',
        KeyCode::KEY_S => 's',
        KeyCode::KEY_T => 't',
        KeyCode::KEY_U => 'u',
        KeyCode::KEY_V => 'v',
        KeyCode::KEY_W => 'w',
        KeyCode::KEY_X => 'x',
        KeyCode::KEY_Y => 'y',
        KeyCode::KEY_Z => 'z',
        KeyCode::KEY_1 => if shift { '!' } else { '1' },
        KeyCode::KEY_2 => if shift { '@' } else { '2' },
        KeyCode::KEY_3 => if shift { '#' } else { '3' },
        KeyCode::KEY_4 => if shift { '$' } else { '4' },
        KeyCode::KEY_5 => if shift { '%' } else { '5' },
        KeyCode::KEY_6 => if shift { '^' } else { '6' },
        KeyCode::KEY_7 => if shift { '&' } else { '7' },
        KeyCode::KEY_8 => if shift { '*' } else { '8' },
        KeyCode::KEY_9 => if shift { '(' } else { '9' },
        KeyCode::KEY_0 => if shift { ')' } else { '0' },
        KeyCode::KEY_SPACE => ' ',
        KeyCode::KEY_ENTER => '\n',
        KeyCode::KEY_TAB => '\t',
        KeyCode::KEY_MINUS => if shift { '_' } else { '-' },
        KeyCode::KEY_EQUAL => if shift { '+' } else { '=' },
        KeyCode::KEY_LEFTBRACE => if shift { '{' } else { '[' },
        KeyCode::KEY_RIGHTBRACE => if shift { '}' } else { ']' },
        KeyCode::KEY_SEMICOLON => if shift { ':' } else { ';' },
        KeyCode::KEY_APOSTROPHE => if shift { '"' } else { '\'' },
        KeyCode::KEY_GRAVE => if shift { '~' } else { '`' },
        KeyCode::KEY_BACKSLASH => if shift { '|' } else { '\\' },
        KeyCode::KEY_COMMA => if shift { '<' } else { ',' },
        KeyCode::KEY_DOT => if shift { '>' } else { '.' },
        KeyCode::KEY_SLASH => if shift { '?' } else { '/' },
        _ => return None,
    };
    
    if shift && lower.is_ascii_lowercase() {
        Some(lower.to_ascii_uppercase())
    } else {
        Some(lower)
    }
}

