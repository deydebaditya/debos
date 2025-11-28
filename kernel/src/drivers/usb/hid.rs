//! USB HID (Human Interface Device) Driver
//!
//! Implements USB HID class for keyboards, mice, and other input devices.
//!
//! ## HID Report Protocol
//! - Boot Protocol: Simple, fixed format for BIOS compatibility
//! - Report Protocol: Full-featured, uses report descriptors
//!
//! ## Supported Devices
//! - Keyboards (boot and report protocol)
//! - Mice (boot and report protocol)
//! - Generic HID devices

use alloc::vec::Vec;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;

use super::{UsbDevice, UsbClass};
use super::descriptor::HidDescriptor;
use crate::drivers::input::{InputEvent, InputEventType, queue_event, KeyCode};

/// HID subclass codes
pub mod HidSubclass {
    pub const NONE: u8 = 0;
    pub const BOOT: u8 = 1;
}

/// HID protocol codes
pub mod HidProtocol {
    pub const NONE: u8 = 0;
    pub const KEYBOARD: u8 = 1;
    pub const MOUSE: u8 = 2;
}

/// HID request types
pub mod HidRequest {
    pub const GET_REPORT: u8 = 0x01;
    pub const GET_IDLE: u8 = 0x02;
    pub const GET_PROTOCOL: u8 = 0x03;
    pub const SET_REPORT: u8 = 0x09;
    pub const SET_IDLE: u8 = 0x0A;
    pub const SET_PROTOCOL: u8 = 0x0B;
}

/// HID report types
pub mod ReportType {
    pub const INPUT: u8 = 1;
    pub const OUTPUT: u8 = 2;
    pub const FEATURE: u8 = 3;
}

/// Boot protocol keyboard report (8 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct BootKeyboardReport {
    /// Modifier keys (Ctrl, Shift, Alt, GUI)
    pub modifiers: u8,
    /// Reserved byte
    pub reserved: u8,
    /// Key codes (up to 6 simultaneous keys)
    pub keys: [u8; 6],
}

impl BootKeyboardReport {
    /// Check if left Ctrl is pressed
    pub fn left_ctrl(&self) -> bool { self.modifiers & 0x01 != 0 }
    /// Check if left Shift is pressed
    pub fn left_shift(&self) -> bool { self.modifiers & 0x02 != 0 }
    /// Check if left Alt is pressed
    pub fn left_alt(&self) -> bool { self.modifiers & 0x04 != 0 }
    /// Check if left GUI (Windows/Command) is pressed
    pub fn left_gui(&self) -> bool { self.modifiers & 0x08 != 0 }
    /// Check if right Ctrl is pressed
    pub fn right_ctrl(&self) -> bool { self.modifiers & 0x10 != 0 }
    /// Check if right Shift is pressed
    pub fn right_shift(&self) -> bool { self.modifiers & 0x20 != 0 }
    /// Check if right Alt is pressed
    pub fn right_alt(&self) -> bool { self.modifiers & 0x40 != 0 }
    /// Check if right GUI is pressed
    pub fn right_gui(&self) -> bool { self.modifiers & 0x80 != 0 }
    
    /// Check if any Ctrl is pressed
    pub fn ctrl(&self) -> bool { self.left_ctrl() || self.right_ctrl() }
    /// Check if any Shift is pressed
    pub fn shift(&self) -> bool { self.left_shift() || self.right_shift() }
    /// Check if any Alt is pressed
    pub fn alt(&self) -> bool { self.left_alt() || self.right_alt() }
}

/// Boot protocol mouse report (3+ bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct BootMouseReport {
    /// Button state (bit 0 = left, bit 1 = right, bit 2 = middle)
    pub buttons: u8,
    /// X movement (signed)
    pub x: i8,
    /// Y movement (signed)
    pub y: i8,
}

impl BootMouseReport {
    pub fn left_button(&self) -> bool { self.buttons & 0x01 != 0 }
    pub fn right_button(&self) -> bool { self.buttons & 0x02 != 0 }
    pub fn middle_button(&self) -> bool { self.buttons & 0x04 != 0 }
}

/// USB HID device
pub struct HidDevice {
    /// USB device info
    pub usb_device: UsbDevice,
    
    /// HID protocol (keyboard or mouse)
    pub protocol: u8,
    
    /// Interface number
    pub interface: u8,
    
    /// Endpoint address for interrupt IN
    pub endpoint: u8,
    
    /// Report length
    pub report_length: u8,
    
    /// Last keyboard report (for key release detection)
    last_keyboard_report: BootKeyboardReport,
    
    /// Last mouse buttons (for button release detection)
    last_mouse_buttons: u8,
}

impl HidDevice {
    pub fn new(usb_device: UsbDevice, protocol: u8, interface: u8, endpoint: u8) -> Self {
        HidDevice {
            usb_device,
            protocol,
            interface,
            endpoint,
            report_length: if protocol == HidProtocol::KEYBOARD { 8 } else { 3 },
            last_keyboard_report: BootKeyboardReport::default(),
            last_mouse_buttons: 0,
        }
    }
    
    /// Process keyboard report
    pub fn process_keyboard_report(&mut self, report: &BootKeyboardReport) {
        let timestamp = crate::scheduler::ticks();
        
        // Check for new key presses
        for &key in &report.keys {
            if key != 0 {
                // Check if this is a new key
                let was_pressed = self.last_keyboard_report.keys.contains(&key);
                if !was_pressed {
                    // Key press event
                    let event = InputEvent {
                        timestamp,
                        event_type: InputEventType::Key,
                        code: key as u16,
                        value: 1, // Key down
                    };
                    queue_event(event);
                }
            }
        }
        
        // Check for key releases
        for &key in &self.last_keyboard_report.keys {
            if key != 0 && !report.keys.contains(&key) {
                // Key release event
                let event = InputEvent {
                    timestamp,
                    event_type: InputEventType::Key,
                    code: key as u16,
                    value: 0, // Key up
                };
                queue_event(event);
            }
        }
        
        // Check modifier changes
        let old_mods = self.last_keyboard_report.modifiers;
        let new_mods = report.modifiers;
        
        if old_mods != new_mods {
            // Report modifier changes
            self.check_modifier_change(timestamp, old_mods, new_mods, 0x01, KeyCode::KEY_LEFTCTRL);
            self.check_modifier_change(timestamp, old_mods, new_mods, 0x02, KeyCode::KEY_LEFTSHIFT);
            self.check_modifier_change(timestamp, old_mods, new_mods, 0x04, KeyCode::KEY_LEFTALT);
            self.check_modifier_change(timestamp, old_mods, new_mods, 0x08, KeyCode::KEY_LEFTMETA);
            self.check_modifier_change(timestamp, old_mods, new_mods, 0x10, KeyCode::KEY_RIGHTCTRL);
            self.check_modifier_change(timestamp, old_mods, new_mods, 0x20, KeyCode::KEY_RIGHTSHIFT);
            self.check_modifier_change(timestamp, old_mods, new_mods, 0x40, KeyCode::KEY_RIGHTALT);
            self.check_modifier_change(timestamp, old_mods, new_mods, 0x80, KeyCode::KEY_RIGHTMETA);
        }
        
        // Save current state
        self.last_keyboard_report = *report;
    }
    
    fn check_modifier_change(&self, timestamp: u64, old: u8, new: u8, mask: u8, code: u16) {
        let was_pressed = old & mask != 0;
        let is_pressed = new & mask != 0;
        
        if was_pressed != is_pressed {
            let event = InputEvent {
                timestamp,
                event_type: InputEventType::Key,
                code,
                value: if is_pressed { 1 } else { 0 },
            };
            queue_event(event);
        }
    }
    
    /// Process mouse report
    pub fn process_mouse_report(&mut self, report: &BootMouseReport) {
        let timestamp = crate::scheduler::ticks();
        
        // Report button changes
        if report.buttons != self.last_mouse_buttons {
            // Left button
            if (report.buttons & 0x01) != (self.last_mouse_buttons & 0x01) {
                queue_event(InputEvent {
                    timestamp,
                    event_type: InputEventType::Key,
                    code: KeyCode::BTN_LEFT,
                    value: if report.left_button() { 1 } else { 0 },
                });
            }
            
            // Right button
            if (report.buttons & 0x02) != (self.last_mouse_buttons & 0x02) {
                queue_event(InputEvent {
                    timestamp,
                    event_type: InputEventType::Key,
                    code: KeyCode::BTN_RIGHT,
                    value: if report.right_button() { 1 } else { 0 },
                });
            }
            
            // Middle button
            if (report.buttons & 0x04) != (self.last_mouse_buttons & 0x04) {
                queue_event(InputEvent {
                    timestamp,
                    event_type: InputEventType::Key,
                    code: KeyCode::BTN_MIDDLE,
                    value: if report.middle_button() { 1 } else { 0 },
                });
            }
            
            self.last_mouse_buttons = report.buttons;
        }
        
        // Report movement
        if report.x != 0 {
            queue_event(InputEvent {
                timestamp,
                event_type: InputEventType::Relative,
                code: 0, // REL_X
                value: report.x as i32,
            });
        }
        
        if report.y != 0 {
            queue_event(InputEvent {
                timestamp,
                event_type: InputEventType::Relative,
                code: 1, // REL_Y
                value: report.y as i32,
            });
        }
    }
    
    /// Check if this is a keyboard
    pub fn is_keyboard(&self) -> bool {
        self.protocol == HidProtocol::KEYBOARD
    }
    
    /// Check if this is a mouse
    pub fn is_mouse(&self) -> bool {
        self.protocol == HidProtocol::MOUSE
    }
}

/// USB HID to scan code translation
pub mod usb_to_scancode {
    /// Convert USB HID keyboard usage ID to ASCII character
    pub fn to_ascii(usage: u8, shift: bool) -> Option<char> {
        match usage {
            // Letters (a-z)
            0x04..=0x1D => {
                let base = if shift { b'A' } else { b'a' };
                Some((base + usage - 0x04) as char)
            }
            // Numbers (1-9, 0)
            0x1E..=0x26 => {
                if shift {
                    Some(match usage {
                        0x1E => '!',
                        0x1F => '@',
                        0x20 => '#',
                        0x21 => '$',
                        0x22 => '%',
                        0x23 => '^',
                        0x24 => '&',
                        0x25 => '*',
                        0x26 => '(',
                        _ => return None,
                    })
                } else {
                    Some(((usage - 0x1E + 1) % 10 + b'0') as char)
                }
            }
            0x27 => Some(if shift { ')' } else { '0' }),
            // Special keys
            0x28 => Some('\n'),  // Enter
            0x29 => Some('\x1B'), // Escape
            0x2A => Some('\x08'), // Backspace
            0x2B => Some('\t'),   // Tab
            0x2C => Some(' '),    // Space
            0x2D => Some(if shift { '_' } else { '-' }),
            0x2E => Some(if shift { '+' } else { '=' }),
            0x2F => Some(if shift { '{' } else { '[' }),
            0x30 => Some(if shift { '}' } else { ']' }),
            0x31 => Some(if shift { '|' } else { '\\' }),
            0x33 => Some(if shift { ':' } else { ';' }),
            0x34 => Some(if shift { '"' } else { '\'' }),
            0x35 => Some(if shift { '~' } else { '`' }),
            0x36 => Some(if shift { '<' } else { ',' }),
            0x37 => Some(if shift { '>' } else { '.' }),
            0x38 => Some(if shift { '?' } else { '/' }),
            _ => None,
        }
    }
}

lazy_static! {
    /// Registered HID devices
    static ref HID_DEVICES: Mutex<Vec<HidDevice>> = Mutex::new(Vec::new());
}

/// Register a HID device
pub fn register_device(device: HidDevice) {
    let protocol = device.protocol;
    HID_DEVICES.lock().push(device);
    
    match protocol {
        HidProtocol::KEYBOARD => crate::println!("    USB HID Keyboard registered"),
        HidProtocol::MOUSE => crate::println!("    USB HID Mouse registered"),
        _ => crate::println!("    USB HID device registered"),
    }
}

/// Get all HID devices
pub fn get_devices() -> Vec<u8> {
    HID_DEVICES.lock()
        .iter()
        .map(|d| d.usb_device.address)
        .collect()
}

/// Find HID keyboards
pub fn find_keyboards() -> usize {
    HID_DEVICES.lock()
        .iter()
        .filter(|d| d.is_keyboard())
        .count()
}

/// Find HID mice
pub fn find_mice() -> usize {
    HID_DEVICES.lock()
        .iter()
        .filter(|d| d.is_mouse())
        .count()
}

/// Initialize HID subsystem
pub fn init() {
    // Find HID devices among enumerated USB devices
    for usb_device in super::get_devices() {
        if usb_device.class == UsbClass::Hid {
            // Determine HID protocol based on subclass
            let protocol = usb_device.protocol;
            let device = HidDevice::new(usb_device, protocol, 0, 0x81);
            register_device(device);
        }
    }
}

