# DebOS Device Subsystem Implementation Plan

> **Phase 2D+: Device Manager, Networking, and I/O**  
> **Goal:** Complete device ecosystem with USB, display, input, and networking  
> **Status:** Planning  
> **Prerequisites:** Phase 1 (Kernel), Phase 2A-C (Filesystem)

---

## Executive Summary

This document outlines the implementation of DebOS's device subsystem, providing:

- **Device Manager**: Central hub for device enumeration and driver management
- **Input Subsystem**: Keyboard, mouse, touchpad support
- **USB Stack**: xHCI controller, device enumeration, standard device classes
- **Display Subsystem**: Framebuffer, basic graphics, text console
- **Networking**: Full TCP/IP stack for internet connectivity
- **Serial I/O**: UART for debugging and serial devices

---

## System Architecture

### Device Subsystem Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                      USER APPLICATIONS                               │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────────────┐ │
│  │ Text Apps  │ │ GUI Apps   │ │ Net Apps   │ │ USB Apps           │ │
│  └────────────┘ └────────────┘ └────────────┘ └────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     DEVICE MANAGER (Ring 3)                          │
│  ┌──────────────────────────────────────────────────────────────────┐│
│  │                        Device Tree                                ││
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────────────┐  ││
│  │  │ PCI    │ │ USB    │ │ ACPI   │ │ DT     │ │ Platform       │  ││
│  │  │ Bus    │ │ Bus    │ │ Enum   │ │ (ARM)  │ │ Devices        │  ││
│  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────────────┘  ││
│  └──────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────────┐
│                     SUBSYSTEM DRIVERS                                │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────┐ │
│  │ Input    │ │ Display  │ │ Network  │ │ Storage  │ │ USB        │ │
│  │ (/dev/   │ │ (/dev/   │ │ (eth0,   │ │ (VirtIO  │ │ (xHCI,     │ │
│  │ input/*, │ │ fb0,     │ │ wlan0)   │ │ NVMe)    │ │ devices)   │ │
│  │ kbd,mouse│ │ tty*)    │ │          │ │          │ │            │ │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     KERNEL (Ring 0)                                  │
│  ┌──────────────────────────────────────────────────────────────────┐│
│  │ IRQ Routing │ DMA Management │ MMIO Mapping │ Capability Grants  ││
│  └──────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

---

## Component Designs

### 1. Device Manager

The Device Manager is the central coordinator for all device operations.

#### 1.1 Device Tree Structure

```rust
/// Device in the device tree
pub struct Device {
    /// Unique device ID
    pub id: DeviceId,
    
    /// Device name (e.g., "usb-hid-keyboard")
    pub name: String,
    
    /// Device class
    pub class: DeviceClass,
    
    /// Bus this device is attached to
    pub bus: BusType,
    
    /// Parent device (None for root devices)
    pub parent: Option<DeviceId>,
    
    /// Child devices
    pub children: Vec<DeviceId>,
    
    /// Device state
    pub state: DeviceState,
    
    /// Assigned driver
    pub driver: Option<DriverId>,
    
    /// Resources (MMIO, IRQ, DMA)
    pub resources: DeviceResources,
}

/// Device classes
pub enum DeviceClass {
    // Storage
    BlockDevice,
    
    // Input
    Keyboard,
    Mouse,
    Touchpad,
    Gamepad,
    
    // Display
    DisplayController,
    Framebuffer,
    
    // Network
    Ethernet,
    Wireless,
    
    // USB
    UsbController,
    UsbHub,
    UsbDevice,
    
    // Serial
    SerialPort,
    
    // Audio
    AudioController,
    
    // Other
    Unknown(u32),
}

/// Bus types
pub enum BusType {
    Root,           // Virtual root
    PCI,            // PCI/PCIe
    USB,            // USB
    Platform,       // Platform/SoC devices
    VirtIO,         // VirtIO (MMIO or PCI)
    I2C,            // I2C bus
    SPI,            // SPI bus
}

/// Device resources
pub struct DeviceResources {
    /// Memory-mapped I/O regions
    pub mmio: Vec<MmioRegion>,
    
    /// Assigned IRQ lines
    pub irqs: Vec<IrqNumber>,
    
    /// DMA channels/buffers
    pub dma: Vec<DmaBuffer>,
    
    /// I/O ports (x86 only)
    pub io_ports: Vec<IoPortRange>,
}
```

#### 1.2 Device Manager API

```rust
/// Device Manager interface
pub trait DeviceManager {
    /// Enumerate all devices on a bus
    fn enumerate_bus(&mut self, bus: BusType) -> Result<Vec<DeviceId>>;
    
    /// Register a new device
    fn register_device(&mut self, device: Device) -> Result<DeviceId>;
    
    /// Unregister a device
    fn unregister_device(&mut self, id: DeviceId) -> Result<()>;
    
    /// Find devices by class
    fn find_by_class(&self, class: DeviceClass) -> Vec<&Device>;
    
    /// Find devices by bus
    fn find_by_bus(&self, bus: BusType) -> Vec<&Device>;
    
    /// Get device by ID
    fn get_device(&self, id: DeviceId) -> Option<&Device>;
    
    /// Bind driver to device
    fn bind_driver(&mut self, device: DeviceId, driver: DriverId) -> Result<()>;
    
    /// Unbind driver from device
    fn unbind_driver(&mut self, device: DeviceId) -> Result<()>;
    
    /// Handle device hotplug
    fn handle_hotplug(&mut self, event: HotplugEvent) -> Result<()>;
}
```

### 2. Input Subsystem

#### 2.1 Input Event Model

```rust
/// Input event (similar to Linux evdev)
pub struct InputEvent {
    /// Timestamp
    pub time: Timestamp,
    
    /// Event type
    pub event_type: InputEventType,
    
    /// Event code (key code, axis, etc.)
    pub code: u16,
    
    /// Event value
    pub value: i32,
}

/// Input event types
pub enum InputEventType {
    /// Synchronization events
    Sync = 0x00,
    
    /// Key/button events
    Key = 0x01,
    
    /// Relative movement (mouse)
    Relative = 0x02,
    
    /// Absolute positioning (touchscreen)
    Absolute = 0x03,
    
    /// Miscellaneous events
    Misc = 0x04,
    
    /// LED control
    Led = 0x11,
    
    /// Force feedback
    ForceFeedback = 0x15,
}

/// Key codes (USB HID compatible)
pub mod KeyCode {
    pub const KEY_RESERVED: u16 = 0;
    pub const KEY_ESC: u16 = 1;
    pub const KEY_1: u16 = 2;
    // ... full key mapping
    pub const KEY_A: u16 = 30;
    pub const KEY_B: u16 = 48;
    // etc.
}

/// Relative axis codes
pub mod RelativeAxis {
    pub const REL_X: u16 = 0x00;
    pub const REL_Y: u16 = 0x01;
    pub const REL_Z: u16 = 0x02;
    pub const REL_WHEEL: u16 = 0x08;
    pub const REL_HWHEEL: u16 = 0x06;
}
```

#### 2.2 Keyboard Driver

```rust
/// Keyboard driver interface
pub trait KeyboardDriver {
    /// Initialize the keyboard
    fn init(&mut self) -> Result<()>;
    
    /// Poll for key events
    fn poll(&mut self) -> Option<InputEvent>;
    
    /// Set LED state (Caps Lock, Num Lock, etc.)
    fn set_leds(&mut self, leds: LedState) -> Result<()>;
    
    /// Get keyboard layout
    fn get_layout(&self) -> KeyboardLayout;
    
    /// Set keyboard layout
    fn set_layout(&mut self, layout: KeyboardLayout) -> Result<()>;
}

/// PS/2 keyboard driver (for x86)
pub struct Ps2Keyboard {
    data_port: u16,
    command_port: u16,
    state: KeyboardState,
}

/// USB HID keyboard driver
pub struct UsbHidKeyboard {
    device: UsbDevice,
    interface: u8,
    endpoint: u8,
    state: KeyboardState,
}

/// VirtIO input driver (for VMs)
pub struct VirtioInput {
    device: VirtioDevice,
    event_queue: VirtQueue,
    status_queue: VirtQueue,
}
```

#### 2.3 Mouse Driver

```rust
/// Mouse driver interface
pub trait MouseDriver {
    /// Initialize the mouse
    fn init(&mut self) -> Result<()>;
    
    /// Poll for mouse events
    fn poll(&mut self) -> Option<InputEvent>;
    
    /// Get mouse capabilities
    fn capabilities(&self) -> MouseCapabilities;
}

/// Mouse capabilities
pub struct MouseCapabilities {
    /// Number of buttons
    pub buttons: u8,
    
    /// Has scroll wheel
    pub scroll_wheel: bool,
    
    /// Has horizontal scroll
    pub horizontal_scroll: bool,
    
    /// Resolution (DPI)
    pub resolution: u16,
}
```

### 3. USB Subsystem

#### 3.1 USB Host Controller

```rust
/// USB Host Controller types
pub enum UsbControllerType {
    /// Universal Host Controller Interface (USB 1.x)
    UHCI,
    
    /// Open Host Controller Interface (USB 1.x)
    OHCI,
    
    /// Enhanced Host Controller Interface (USB 2.0)
    EHCI,
    
    /// Extensible Host Controller Interface (USB 3.x)
    XHCI,
}

/// USB Host Controller interface
pub trait UsbHostController {
    /// Initialize the controller
    fn init(&mut self) -> Result<()>;
    
    /// Reset the controller
    fn reset(&mut self) -> Result<()>;
    
    /// Enumerate connected devices
    fn enumerate(&mut self) -> Result<Vec<UsbDevice>>;
    
    /// Submit a USB transfer
    fn submit_transfer(&mut self, transfer: UsbTransfer) -> Result<TransferId>;
    
    /// Cancel a pending transfer
    fn cancel_transfer(&mut self, id: TransferId) -> Result<()>;
    
    /// Get port status
    fn port_status(&self, port: u8) -> PortStatus;
    
    /// Reset a port
    fn reset_port(&mut self, port: u8) -> Result<()>;
}

/// xHCI Controller implementation
pub struct XhciController {
    /// Base MMIO address
    base: usize,
    
    /// Capability registers
    cap_regs: XhciCapabilityRegs,
    
    /// Operational registers
    op_regs: XhciOperationalRegs,
    
    /// Runtime registers
    rt_regs: XhciRuntimeRegs,
    
    /// Device context base array
    dcbaa: *mut u64,
    
    /// Command ring
    command_ring: XhciRing,
    
    /// Event ring
    event_ring: XhciRing,
    
    /// Transfer rings (per device/endpoint)
    transfer_rings: BTreeMap<(u8, u8), XhciRing>,
    
    /// Scratchpad buffers
    scratchpad: Vec<*mut u8>,
}
```

#### 3.2 USB Device Classes

```rust
/// USB device class codes
pub enum UsbDeviceClass {
    /// Per-interface class
    PerInterface = 0x00,
    
    /// Audio devices
    Audio = 0x01,
    
    /// Communication devices (modems, serial)
    Communication = 0x02,
    
    /// Human Interface Devices (keyboard, mouse)
    Hid = 0x03,
    
    /// Physical devices (force feedback)
    Physical = 0x05,
    
    /// Image devices (cameras, scanners)
    Image = 0x06,
    
    /// Printers
    Printer = 0x07,
    
    /// Mass storage (USB drives)
    MassStorage = 0x08,
    
    /// USB hubs
    Hub = 0x09,
    
    /// Smart cards
    SmartCard = 0x0B,
    
    /// Video devices
    Video = 0x0E,
    
    /// Personal healthcare
    PersonalHealthcare = 0x0F,
    
    /// Audio/Video devices
    AudioVideo = 0x10,
    
    /// Vendor specific
    VendorSpecific = 0xFF,
}

/// USB Mass Storage driver (for USB drives)
pub struct UsbMassStorage {
    device: UsbDevice,
    interface: u8,
    bulk_in: u8,
    bulk_out: u8,
    max_lun: u8,
}

impl BlockDevice for UsbMassStorage {
    fn read_sector(&self, sector: u64, buf: &mut [u8]) -> Result<()>;
    fn write_sector(&self, sector: u64, buf: &[u8]) -> Result<()>;
    fn sector_size(&self) -> usize;
    fn sector_count(&self) -> u64;
}
```

### 4. Display Subsystem

#### 4.1 Framebuffer Interface

```rust
/// Framebuffer information
pub struct FramebufferInfo {
    /// Physical address of framebuffer
    pub physical_address: usize,
    
    /// Virtual address (after mapping)
    pub virtual_address: usize,
    
    /// Width in pixels
    pub width: u32,
    
    /// Height in pixels
    pub height: u32,
    
    /// Pitch (bytes per row)
    pub pitch: u32,
    
    /// Bits per pixel
    pub bpp: u8,
    
    /// Pixel format
    pub format: PixelFormat,
}

/// Pixel formats
pub enum PixelFormat {
    RGB888,
    RGBA8888,
    BGR888,
    BGRA8888,
    RGB565,
}

/// Framebuffer driver interface
pub trait FramebufferDriver {
    /// Get framebuffer info
    fn info(&self) -> &FramebufferInfo;
    
    /// Get raw framebuffer pointer
    fn buffer(&mut self) -> &mut [u8];
    
    /// Set a single pixel
    fn set_pixel(&mut self, x: u32, y: u32, color: Color);
    
    /// Fill a rectangle
    fn fill_rect(&mut self, rect: Rect, color: Color);
    
    /// Blit an image
    fn blit(&mut self, x: u32, y: u32, image: &Image);
    
    /// Scroll the display
    fn scroll(&mut self, lines: i32);
    
    /// Flush changes to display (for double buffering)
    fn flush(&mut self);
}

/// Color representation
#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub const RED: Color = Color { r: 255, g: 0, b: 0, a: 255 };
    pub const GREEN: Color = Color { r: 0, g: 255, b: 0, a: 255 };
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255, a: 255 };
}
```

#### 4.2 Text Console

```rust
/// Text console over framebuffer
pub struct TextConsole {
    fb: Box<dyn FramebufferDriver>,
    font: Font,
    columns: u32,
    rows: u32,
    cursor_x: u32,
    cursor_y: u32,
    fg_color: Color,
    bg_color: Color,
    buffer: Vec<Vec<char>>,
}

impl TextConsole {
    /// Write a character at cursor position
    pub fn put_char(&mut self, c: char);
    
    /// Write a string
    pub fn write_str(&mut self, s: &str);
    
    /// Move cursor
    pub fn move_cursor(&mut self, x: u32, y: u32);
    
    /// Clear screen
    pub fn clear(&mut self);
    
    /// Scroll up
    pub fn scroll_up(&mut self, lines: u32);
    
    /// Set foreground color
    pub fn set_fg(&mut self, color: Color);
    
    /// Set background color
    pub fn set_bg(&mut self, color: Color);
}
```

### 5. Networking Stack

#### 5.1 Network Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                      SOCKET API (libdebos)                           │
│  socket(), bind(), listen(), accept(), connect(), send(), recv()     │
└─────────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────────┐
│                      NET SERVER (Ring 3)                             │
│  ┌──────────────────────────────────────────────────────────────────┐│
│  │                      Socket Layer                                 ││
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────────┐  ││
│  │  │ TCP      │ │ UDP      │ │ Raw      │ │ Unix Domain        │  ││
│  │  │ Sockets  │ │ Sockets  │ │ Sockets  │ │ Sockets            │  ││
│  │  └──────────┘ └──────────┘ └──────────┘ └────────────────────┘  ││
│  └──────────────────────────────────────────────────────────────────┘│
│  ┌──────────────────────────────────────────────────────────────────┐│
│  │                    Transport Layer                                ││
│  │  ┌──────────────────────────┐ ┌──────────────────────────────┐  ││
│  │  │ TCP (Full State Machine) │ │ UDP (Connectionless)         │  ││
│  │  └──────────────────────────┘ └──────────────────────────────┘  ││
│  └──────────────────────────────────────────────────────────────────┘│
│  ┌──────────────────────────────────────────────────────────────────┐│
│  │                    Network Layer                                  ││
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────────┐  ││
│  │  │ IPv4     │ │ IPv6     │ │ ICMP     │ │ Routing            │  ││
│  │  └──────────┘ └──────────┘ └──────────┘ └────────────────────┘  ││
│  └──────────────────────────────────────────────────────────────────┘│
│  ┌──────────────────────────────────────────────────────────────────┐│
│  │                    Link Layer                                     ││
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────────┐  ││
│  │  │ ARP      │ │ NDP      │ │ Ethernet │ │ Loopback           │  ││
│  │  └──────────┘ └──────────┘ └──────────┘ └────────────────────┘  ││
│  └──────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────────┐
│                    NETWORK DRIVERS                                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────────────┐ │
│  │ VirtIO-  │ │ e1000    │ │ RTL8139  │ │ Intel i210/i219       │ │
│  │ Net      │ │          │ │          │ │                       │ │
│  └──────────┘ └──────────┘ └──────────┘ └────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

#### 5.2 Socket API

```rust
/// Socket domain (address family)
pub enum SocketDomain {
    /// IPv4 Internet protocols
    Inet = 2,
    /// IPv6 Internet protocols
    Inet6 = 10,
    /// Unix domain sockets
    Unix = 1,
}

/// Socket type
pub enum SocketType {
    /// Stream socket (TCP)
    Stream = 1,
    /// Datagram socket (UDP)
    Dgram = 2,
    /// Raw socket
    Raw = 3,
}

/// Socket address (IPv4)
#[repr(C)]
pub struct SocketAddrV4 {
    pub family: u16,
    pub port: u16,
    pub addr: [u8; 4],
    pub zero: [u8; 8],
}

/// Socket address (IPv6)
#[repr(C)]
pub struct SocketAddrV6 {
    pub family: u16,
    pub port: u16,
    pub flowinfo: u32,
    pub addr: [u8; 16],
    pub scope_id: u32,
}

/// Socket operations
pub trait Socket {
    fn bind(&mut self, addr: &SocketAddr) -> Result<()>;
    fn listen(&mut self, backlog: i32) -> Result<()>;
    fn accept(&mut self) -> Result<(Box<dyn Socket>, SocketAddr)>;
    fn connect(&mut self, addr: &SocketAddr) -> Result<()>;
    fn send(&mut self, buf: &[u8], flags: i32) -> Result<usize>;
    fn recv(&mut self, buf: &mut [u8], flags: i32) -> Result<usize>;
    fn sendto(&mut self, buf: &[u8], flags: i32, addr: &SocketAddr) -> Result<usize>;
    fn recvfrom(&mut self, buf: &mut [u8], flags: i32) -> Result<(usize, SocketAddr)>;
    fn close(&mut self) -> Result<()>;
    fn set_option(&mut self, option: SocketOption, value: &[u8]) -> Result<()>;
    fn get_option(&self, option: SocketOption) -> Result<Vec<u8>>;
}
```

#### 5.3 TCP Implementation

```rust
/// TCP connection state
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

/// TCP Control Block
pub struct TcpControlBlock {
    /// Local address
    pub local_addr: SocketAddrV4,
    /// Remote address
    pub remote_addr: SocketAddrV4,
    /// Current state
    pub state: TcpState,
    /// Send sequence number
    pub snd_nxt: u32,
    /// Send unacknowledged
    pub snd_una: u32,
    /// Send window
    pub snd_wnd: u16,
    /// Receive next
    pub rcv_nxt: u32,
    /// Receive window
    pub rcv_wnd: u16,
    /// Retransmission timeout
    pub rto: Duration,
    /// Round-trip time
    pub rtt: Duration,
    /// Send buffer
    pub send_buf: VecDeque<u8>,
    /// Receive buffer
    pub recv_buf: VecDeque<u8>,
    /// Retransmission queue
    pub retx_queue: VecDeque<TcpSegment>,
}

/// TCP segment header
#[repr(C, packed)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub data_offset_flags: u16,
    pub window: u16,
    pub checksum: u16,
    pub urgent_ptr: u16,
}
```

### 6. Serial I/O (UART)

#### 6.1 UART Driver

```rust
/// UART configuration
pub struct UartConfig {
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: FlowControl,
}

pub enum DataBits { Five, Six, Seven, Eight }
pub enum StopBits { One, Two }
pub enum Parity { None, Odd, Even }
pub enum FlowControl { None, Hardware, Software }

/// UART driver interface
pub trait UartDriver {
    fn init(&mut self, config: &UartConfig) -> Result<()>;
    fn write_byte(&mut self, byte: u8) -> Result<()>;
    fn read_byte(&mut self) -> Result<u8>;
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn set_config(&mut self, config: &UartConfig) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}

/// x86 16550 UART
pub struct Uart16550 {
    base: u16,
    config: UartConfig,
}

/// ARM PL011 UART (already implemented)
pub struct Pl011Uart {
    base: usize,
    config: UartConfig,
}
```

---

## Implementation Phases

### Phase 2D-1: Device Manager Core (Week 1)

| Task | Description | Priority |
|------|-------------|----------|
| DEV-001 | Device struct and DeviceId | Critical |
| DEV-002 | Device tree with parent/child relationships | Critical |
| DEV-003 | Bus abstraction (BusType enum) | Critical |
| DEV-004 | Device resources (MMIO, IRQ, DMA) | Critical |
| DEV-005 | Driver binding mechanism | High |
| DEV-006 | Hotplug event handling | Medium |

### Phase 2D-2: PCI/PCIe Enumeration (Week 1-2)

| Task | Description | Priority |
|------|-------------|----------|
| PCI-001 | PCI configuration space access | Critical |
| PCI-002 | Device enumeration (bus scan) | Critical |
| PCI-003 | BAR parsing and resource allocation | Critical |
| PCI-004 | MSI/MSI-X interrupt support | High |
| PCI-005 | PCIe extended capabilities | Medium |

### Phase 2D-3: Input Subsystem (Week 2)

| Task | Description | Priority |
|------|-------------|----------|
| INPUT-001 | Input event model (evdev-like) | Critical |
| INPUT-002 | Keyboard scancode translation | Critical |
| INPUT-003 | PS/2 keyboard driver (x86) | High |
| INPUT-004 | VirtIO input driver (VM) | High |
| INPUT-005 | Mouse driver | High |
| INPUT-006 | Keyboard layout support | Medium |

### Phase 2D-4: USB Subsystem (Week 2-3)

| Task | Description | Priority |
|------|-------------|----------|
| USB-001 | xHCI controller driver | Critical |
| USB-002 | USB device enumeration | Critical |
| USB-003 | USB descriptor parsing | Critical |
| USB-004 | USB HID driver (keyboard/mouse) | High |
| USB-005 | USB Mass Storage driver | High |
| USB-006 | USB hub support | Medium |

### Phase 2D-5: Display Subsystem (Week 3)

| Task | Description | Priority |
|------|-------------|----------|
| DISP-001 | Framebuffer abstraction | Critical |
| DISP-002 | VirtIO-GPU driver | High |
| DISP-003 | Text console over framebuffer | High |
| DISP-004 | Basic 2D graphics primitives | Medium |
| DISP-005 | Font rendering | Medium |

### Phase 2D-6: Networking Stack (Week 3-4)

| Task | Description | Priority |
|------|-------------|----------|
| NET-001 | Network interface abstraction | Critical |
| NET-002 | Ethernet frame handling | Critical |
| NET-003 | ARP protocol | Critical |
| NET-004 | IPv4 implementation | Critical |
| NET-005 | ICMP (ping) | High |
| NET-006 | UDP implementation | High |
| NET-007 | TCP implementation | High |
| NET-008 | Socket API | High |
| NET-009 | VirtIO-Net driver | High |
| NET-010 | DHCP client | Medium |
| NET-011 | DNS resolver | Medium |

### Phase 2D-7: Serial I/O (Week 4)

| Task | Description | Priority |
|------|-------------|----------|
| SERIAL-001 | UART driver abstraction | High |
| SERIAL-002 | 16550 UART driver (x86) | High |
| SERIAL-003 | PL011 UART driver (ARM) | Done |
| SERIAL-004 | Serial console | High |
| SERIAL-005 | USB-to-serial support | Medium |

---

## Hardware Support Matrix

### Architecture Support

| Device Type | x86_64 | AArch64 | QEMU virt |
|-------------|--------|---------|-----------|
| Keyboard | PS/2, USB | USB | VirtIO |
| Mouse | PS/2, USB | USB | VirtIO |
| Storage | VirtIO, NVMe | VirtIO | VirtIO |
| Network | VirtIO, e1000 | VirtIO | VirtIO |
| Display | VirtIO-GPU, VGA | VirtIO-GPU | VirtIO-GPU |
| Serial | 16550 UART | PL011 | PL011 |
| USB | xHCI | xHCI | QEMU xHCI |

### QEMU Device Support

```bash
# Run DebOS with full device support
qemu-system-aarch64 \
    -machine virt \
    -cpu cortex-a72 \
    -m 512M \
    -nographic \
    -kernel target/aarch64-unknown-none/release/debos-kernel \
    # Storage
    -device virtio-blk-device,drive=hd0 \
    -drive file=disk.img,format=raw,id=hd0 \
    # Networking
    -device virtio-net-device,netdev=net0 \
    -netdev user,id=net0,hostfwd=tcp::2222-:22 \
    # Input
    -device virtio-keyboard-device \
    -device virtio-mouse-device \
    # Display (for graphical mode)
    -device virtio-gpu-device \
    # USB
    -device qemu-xhci \
    -device usb-kbd \
    -device usb-mouse
```

---

## File Structure

```
kernel/src/drivers/
├── mod.rs              # Driver subsystem init
├── device/
│   ├── mod.rs          # Device manager
│   ├── tree.rs         # Device tree
│   ├── resources.rs    # Device resources
│   └── hotplug.rs      # Hotplug handling
├── bus/
│   ├── mod.rs          # Bus abstraction
│   ├── pci.rs          # PCI/PCIe enumeration
│   └── platform.rs     # Platform devices
├── input/
│   ├── mod.rs          # Input subsystem
│   ├── event.rs        # Input events
│   ├── keyboard.rs     # Keyboard driver
│   ├── mouse.rs        # Mouse driver
│   └── virtio_input.rs # VirtIO input
├── usb/
│   ├── mod.rs          # USB subsystem
│   ├── xhci.rs         # xHCI controller
│   ├── descriptor.rs   # USB descriptors
│   ├── hid.rs          # USB HID driver
│   └── mass_storage.rs # USB storage
├── display/
│   ├── mod.rs          # Display subsystem
│   ├── framebuffer.rs  # Framebuffer abstraction
│   ├── console.rs      # Text console
│   ├── virtio_gpu.rs   # VirtIO GPU
│   └── font.rs         # Font rendering
├── net/
│   ├── mod.rs          # Network subsystem
│   ├── interface.rs    # Network interface
│   ├── ethernet.rs     # Ethernet framing
│   ├── arp.rs          # ARP protocol
│   ├── ipv4.rs         # IPv4 protocol
│   ├── icmp.rs         # ICMP protocol
│   ├── udp.rs          # UDP protocol
│   ├── tcp.rs          # TCP protocol
│   ├── socket.rs       # Socket abstraction
│   └── virtio_net.rs   # VirtIO network
└── serial/
    ├── mod.rs          # Serial subsystem
    ├── uart.rs         # UART abstraction
    ├── uart_16550.rs   # x86 UART
    └── pl011.rs        # ARM UART (existing)
```

---

## Success Criteria

| Feature | Test |
|---------|------|
| Device Manager | Enumerate PCI/platform devices, show device tree |
| Keyboard | Type characters in shell |
| Mouse | Report mouse movement and clicks |
| USB | Detect and enumerate USB devices |
| Display | Show framebuffer graphics |
| Networking | `ping 8.8.8.8` returns response |
| TCP | Connect to remote server |
| Serial | Send/receive over serial port |

---

*This document will be updated as implementation progresses.*

