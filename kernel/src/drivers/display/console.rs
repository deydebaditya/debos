//! Framebuffer Text Console
//!
//! Provides text rendering over a framebuffer display.
//! Uses a built-in 8x16 bitmap font.

use spin::Mutex;
use lazy_static::lazy_static;
use alloc::string::String;

/// Console colors (VGA-compatible palette)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    LightMagenta = 13,
    Yellow = 14,
    White = 15,
}

impl Color {
    /// Convert to RGB values
    pub fn to_rgb(self) -> (u8, u8, u8) {
        match self {
            Color::Black => (0, 0, 0),
            Color::Blue => (0, 0, 170),
            Color::Green => (0, 170, 0),
            Color::Cyan => (0, 170, 170),
            Color::Red => (170, 0, 0),
            Color::Magenta => (170, 0, 170),
            Color::Brown => (170, 85, 0),
            Color::LightGray => (170, 170, 170),
            Color::DarkGray => (85, 85, 85),
            Color::LightBlue => (85, 85, 255),
            Color::LightGreen => (85, 255, 85),
            Color::LightCyan => (85, 255, 255),
            Color::LightRed => (255, 85, 85),
            Color::LightMagenta => (255, 85, 255),
            Color::Yellow => (255, 255, 85),
            Color::White => (255, 255, 255),
        }
    }
}

/// Character cell
#[derive(Debug, Clone, Copy)]
pub struct Cell {
    pub character: char,
    pub foreground: Color,
    pub background: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            character: ' ',
            foreground: Color::LightGray,
            background: Color::Black,
        }
    }
}

/// Console font (8x16 bitmap)
pub mod font {
    /// Font width in pixels
    pub const WIDTH: u32 = 8;
    /// Font height in pixels
    pub const HEIGHT: u32 = 16;
    
    /// Get font bitmap for a character (returns 16 bytes, one per row)
    pub fn get_glyph(c: char) -> [u8; 16] {
        // Simple built-in font for printable ASCII
        match c as u8 {
            // Space
            0x20 => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            // !
            0x21 => [0x00, 0x00, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18,
                     0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00],
            // 0-9
            0x30 => [0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xCE, 0xDE, 0xF6,
                     0xE6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
            0x31 => [0x00, 0x00, 0x18, 0x38, 0x78, 0x18, 0x18, 0x18,
                     0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00, 0x00],
            0x32 => [0x00, 0x00, 0x7C, 0xC6, 0x06, 0x0C, 0x18, 0x30,
                     0x60, 0xC0, 0xC6, 0xFE, 0x00, 0x00, 0x00, 0x00],
            0x33 => [0x00, 0x00, 0x7C, 0xC6, 0x06, 0x06, 0x3C, 0x06,
                     0x06, 0x06, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
            0x34 => [0x00, 0x00, 0x0C, 0x1C, 0x3C, 0x6C, 0xCC, 0xFE,
                     0x0C, 0x0C, 0x0C, 0x1E, 0x00, 0x00, 0x00, 0x00],
            0x35 => [0x00, 0x00, 0xFE, 0xC0, 0xC0, 0xC0, 0xFC, 0x06,
                     0x06, 0x06, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
            0x36 => [0x00, 0x00, 0x38, 0x60, 0xC0, 0xC0, 0xFC, 0xC6,
                     0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
            0x37 => [0x00, 0x00, 0xFE, 0xC6, 0x06, 0x06, 0x0C, 0x18,
                     0x30, 0x30, 0x30, 0x30, 0x00, 0x00, 0x00, 0x00],
            0x38 => [0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0x7C, 0xC6,
                     0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
            0x39 => [0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0x7E, 0x06,
                     0x06, 0x06, 0x0C, 0x78, 0x00, 0x00, 0x00, 0x00],
            // A-Z (uppercase)
            0x41..=0x5A => {
                // Simplified uppercase letters
                let idx = c as u8 - 0x41;
                get_uppercase_glyph(idx)
            }
            // a-z (lowercase)
            0x61..=0x7A => {
                // Simplified lowercase letters
                let idx = c as u8 - 0x61;
                get_lowercase_glyph(idx)
            }
            // Default: box character
            _ => [0xFF, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81,
                  0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0xFF],
        }
    }
    
    fn get_uppercase_glyph(idx: u8) -> [u8; 16] {
        match idx {
            0 => [0x00, 0x00, 0x10, 0x38, 0x6C, 0xC6, 0xC6, 0xFE,  // A
                  0xC6, 0xC6, 0xC6, 0xC6, 0x00, 0x00, 0x00, 0x00],
            1 => [0x00, 0x00, 0xFC, 0x66, 0x66, 0x66, 0x7C, 0x66,  // B
                  0x66, 0x66, 0x66, 0xFC, 0x00, 0x00, 0x00, 0x00],
            2 => [0x00, 0x00, 0x3C, 0x66, 0xC2, 0xC0, 0xC0, 0xC0,  // C
                  0xC0, 0xC2, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00],
            3 => [0x00, 0x00, 0xF8, 0x6C, 0x66, 0x66, 0x66, 0x66,  // D
                  0x66, 0x66, 0x6C, 0xF8, 0x00, 0x00, 0x00, 0x00],
            4 => [0x00, 0x00, 0xFE, 0x66, 0x62, 0x68, 0x78, 0x68,  // E
                  0x60, 0x62, 0x66, 0xFE, 0x00, 0x00, 0x00, 0x00],
            5 => [0x00, 0x00, 0xFE, 0x66, 0x62, 0x68, 0x78, 0x68,  // F
                  0x60, 0x60, 0x60, 0xF0, 0x00, 0x00, 0x00, 0x00],
            _ => [0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6,  // Default O-like
                  0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
        }
    }
    
    fn get_lowercase_glyph(idx: u8) -> [u8; 16] {
        match idx {
            0 => [0x00, 0x00, 0x00, 0x00, 0x00, 0x78, 0x0C, 0x7C,  // a
                  0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00, 0x00, 0x00],
            1 => [0x00, 0x00, 0xE0, 0x60, 0x60, 0x78, 0x6C, 0x66,  // b
                  0x66, 0x66, 0x66, 0x7C, 0x00, 0x00, 0x00, 0x00],
            2 => [0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0xC6, 0xC0,  // c
                  0xC0, 0xC0, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
            3 => [0x00, 0x00, 0x1C, 0x0C, 0x0C, 0x3C, 0x6C, 0xCC,  // d
                  0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00, 0x00, 0x00],
            4 => [0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0xC6, 0xFE,  // e
                  0xC0, 0xC0, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
            _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0xC6, 0xC6,  // Default o-like
                  0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
        }
    }
}

/// Framebuffer console state
pub struct Console {
    /// Screen width in characters
    cols: u32,
    /// Screen height in characters
    rows: u32,
    /// Current cursor column
    cursor_col: u32,
    /// Current cursor row
    cursor_row: u32,
    /// Current foreground color
    foreground: Color,
    /// Current background color
    background: Color,
    /// Screen buffer (character cells)
    buffer: alloc::vec::Vec<Cell>,
    /// Pixel width
    pixel_width: u32,
    /// Pixel height
    pixel_height: u32,
}

impl Console {
    /// Create a new console
    pub fn new(pixel_width: u32, pixel_height: u32) -> Self {
        let cols = pixel_width / font::WIDTH;
        let rows = pixel_height / font::HEIGHT;
        let buffer_size = (cols * rows) as usize;
        
        Console {
            cols,
            rows,
            cursor_col: 0,
            cursor_row: 0,
            foreground: Color::LightGray,
            background: Color::Black,
            buffer: alloc::vec![Cell::default(); buffer_size],
            pixel_width,
            pixel_height,
        }
    }
    
    /// Clear the screen
    pub fn clear(&mut self) {
        let cell = Cell {
            character: ' ',
            foreground: self.foreground,
            background: self.background,
        };
        
        for c in self.buffer.iter_mut() {
            *c = cell;
        }
        
        self.cursor_col = 0;
        self.cursor_row = 0;
        
        // Clear framebuffer
        let (r, g, b) = self.background.to_rgb();
        crate::drivers::virtio::gpu::clear(r, g, b);
    }
    
    /// Set cursor position
    pub fn set_cursor(&mut self, col: u32, row: u32) {
        if col < self.cols && row < self.rows {
            self.cursor_col = col;
            self.cursor_row = row;
        }
    }
    
    /// Get cursor position
    pub fn cursor(&self) -> (u32, u32) {
        (self.cursor_col, self.cursor_row)
    }
    
    /// Set colors
    pub fn set_colors(&mut self, fg: Color, bg: Color) {
        self.foreground = fg;
        self.background = bg;
    }
    
    /// Write a character at current position
    pub fn put_char(&mut self, c: char) {
        match c {
            '\n' => {
                self.cursor_col = 0;
                self.cursor_row += 1;
            }
            '\r' => {
                self.cursor_col = 0;
            }
            '\t' => {
                let tab = 8 - (self.cursor_col % 8);
                for _ in 0..tab {
                    self.put_char(' ');
                }
                return;
            }
            '\x08' => {
                // Backspace
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                    self.put_char_at(' ', self.cursor_col, self.cursor_row);
                }
                return;
            }
            c if c >= ' ' => {
                self.put_char_at(c, self.cursor_col, self.cursor_row);
                self.cursor_col += 1;
            }
            _ => {}
        }
        
        // Handle line wrap
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
        }
        
        // Handle scroll
        if self.cursor_row >= self.rows {
            self.scroll_up();
            self.cursor_row = self.rows - 1;
        }
    }
    
    /// Put character at specific position
    fn put_char_at(&mut self, c: char, col: u32, row: u32) {
        if col >= self.cols || row >= self.rows {
            return;
        }
        
        let idx = (row * self.cols + col) as usize;
        self.buffer[idx] = Cell {
            character: c,
            foreground: self.foreground,
            background: self.background,
        };
        
        // Render to framebuffer
        self.render_cell(col, row);
    }
    
    /// Render a cell to the framebuffer
    fn render_cell(&self, col: u32, row: u32) {
        let idx = (row * self.cols + col) as usize;
        let cell = &self.buffer[idx];
        
        let glyph = font::get_glyph(cell.character);
        let (fg_r, fg_g, fg_b) = cell.foreground.to_rgb();
        let (bg_r, bg_g, bg_b) = cell.background.to_rgb();
        
        let px = col * font::WIDTH;
        let py = row * font::HEIGHT;
        
        for (y, &row_bits) in glyph.iter().enumerate() {
            for x in 0..8 {
                let bit = (row_bits >> (7 - x)) & 1;
                if bit != 0 {
                    crate::drivers::virtio::gpu::set_pixel(px + x, py + y as u32, fg_r, fg_g, fg_b);
                } else {
                    crate::drivers::virtio::gpu::set_pixel(px + x, py + y as u32, bg_r, bg_g, bg_b);
                }
            }
        }
    }
    
    /// Scroll up one line
    fn scroll_up(&mut self) {
        // Move buffer contents up
        for row in 1..self.rows {
            for col in 0..self.cols {
                let src_idx = (row * self.cols + col) as usize;
                let dst_idx = ((row - 1) * self.cols + col) as usize;
                self.buffer[dst_idx] = self.buffer[src_idx];
            }
        }
        
        // Clear last row
        let last_row = self.rows - 1;
        for col in 0..self.cols {
            let idx = (last_row * self.cols + col) as usize;
            self.buffer[idx] = Cell::default();
        }
        
        // Re-render entire screen
        self.render_all();
    }
    
    /// Render all cells
    fn render_all(&self) {
        for row in 0..self.rows {
            for col in 0..self.cols {
                self.render_cell(col, row);
            }
        }
        
        crate::drivers::virtio::gpu::flush();
    }
    
    /// Write a string
    pub fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            self.put_char(c);
        }
    }
    
    /// Get dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.cols, self.rows)
    }
}

lazy_static! {
    /// Global console instance
    static ref CONSOLE: Mutex<Option<Console>> = Mutex::new(None);
}

/// Initialize console
pub fn init(width: u32, height: u32) {
    let mut console = Console::new(width, height);
    console.clear();
    *CONSOLE.lock() = Some(console);
}

/// Clear screen
pub fn clear() {
    if let Some(console) = CONSOLE.lock().as_mut() {
        console.clear();
    }
}

/// Write string to console
pub fn write_str(s: &str) {
    if let Some(console) = CONSOLE.lock().as_mut() {
        console.write_str(s);
    }
}

/// Write character to console
pub fn put_char(c: char) {
    if let Some(console) = CONSOLE.lock().as_mut() {
        console.put_char(c);
    }
}

/// Set cursor position
pub fn set_cursor(col: u32, row: u32) {
    if let Some(console) = CONSOLE.lock().as_mut() {
        console.set_cursor(col, row);
    }
}

/// Get cursor position
pub fn cursor() -> Option<(u32, u32)> {
    CONSOLE.lock().as_ref().map(|c| c.cursor())
}

/// Set colors
pub fn set_colors(fg: Color, bg: Color) {
    if let Some(console) = CONSOLE.lock().as_mut() {
        console.set_colors(fg, bg);
    }
}

/// Check if console is available
pub fn is_available() -> bool {
    CONSOLE.lock().is_some()
}

/// Get console dimensions (cols, rows)
pub fn dimensions() -> Option<(u32, u32)> {
    CONSOLE.lock().as_ref().map(|c| c.dimensions())
}

/// Implement core::fmt::Write for console
pub struct ConsoleWriter;

impl core::fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write_str(s);
        Ok(())
    }
}

