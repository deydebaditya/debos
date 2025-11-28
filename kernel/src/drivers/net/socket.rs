//! Socket API
//!
//! BSD-style socket interface for network programming.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU32, Ordering};

use super::{Ipv4Address, MacAddress};

/// Socket domain (address family)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketDomain {
    /// IPv4 Internet protocols
    Inet = 2,
    /// IPv6 Internet protocols
    Inet6 = 10,
    /// Unix domain sockets
    Unix = 1,
}

/// Socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    /// Stream socket (TCP)
    Stream = 1,
    /// Datagram socket (UDP)
    Dgram = 2,
    /// Raw socket
    Raw = 3,
}

/// Socket address (IPv4)
#[derive(Debug, Clone, Copy)]
pub struct SocketAddrV4 {
    pub port: u16,
    pub addr: Ipv4Address,
}

impl SocketAddrV4 {
    pub fn new(addr: Ipv4Address, port: u16) -> Self {
        SocketAddrV4 { addr, port }
    }
    
    pub fn any(port: u16) -> Self {
        SocketAddrV4 {
            addr: Ipv4Address::UNSPECIFIED,
            port,
        }
    }
}

/// Socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Closed,
    Bound,
    Listening,
    Connected,
    Connecting,
}

/// Socket file descriptor
pub type SocketFd = u32;

/// Socket
pub struct Socket {
    pub fd: SocketFd,
    pub domain: SocketDomain,
    pub socket_type: SocketType,
    pub state: SocketState,
    pub local_addr: Option<SocketAddrV4>,
    pub remote_addr: Option<SocketAddrV4>,
    pub recv_buffer: Vec<u8>,
    pub send_buffer: Vec<u8>,
    pub backlog: u32,
    pub pending_connections: Vec<SocketFd>,
}

impl Socket {
    pub fn new(fd: SocketFd, domain: SocketDomain, socket_type: SocketType) -> Self {
        Socket {
            fd,
            domain,
            socket_type,
            state: SocketState::Closed,
            local_addr: None,
            remote_addr: None,
            recv_buffer: Vec::new(),
            send_buffer: Vec::new(),
            backlog: 0,
            pending_connections: Vec::new(),
        }
    }
}

/// Next socket file descriptor
static NEXT_SOCKET_FD: AtomicU32 = AtomicU32::new(3); // 0, 1, 2 reserved for stdin/stdout/stderr

lazy_static! {
    /// All sockets
    static ref SOCKETS: Mutex<BTreeMap<SocketFd, Socket>> = Mutex::new(BTreeMap::new());
}

/// Initialize socket subsystem
pub fn init() {
    // Nothing to initialize for now
}

/// Create a new socket
pub fn socket(domain: SocketDomain, socket_type: SocketType) -> Result<SocketFd, &'static str> {
    let fd = NEXT_SOCKET_FD.fetch_add(1, Ordering::Relaxed);
    let sock = Socket::new(fd, domain, socket_type);
    
    SOCKETS.lock().insert(fd, sock);
    
    Ok(fd)
}

/// Bind socket to address
pub fn bind(fd: SocketFd, addr: SocketAddrV4) -> Result<(), &'static str> {
    let mut sockets = SOCKETS.lock();
    
    // First check if socket exists and is in correct state
    {
        let sock = sockets.get(&fd).ok_or("Invalid socket")?;
        if sock.state != SocketState::Closed {
            return Err("Socket already bound");
        }
    }
    
    // Check if port is already in use
    for (other_fd, other) in sockets.iter() {
        if let Some(local) = &other.local_addr {
            if local.port == addr.port && *other_fd != fd {
                return Err("Address already in use");
            }
        }
    }
    
    // Now update the socket
    let sock = sockets.get_mut(&fd).ok_or("Invalid socket")?;
    sock.local_addr = Some(addr);
    sock.state = SocketState::Bound;
    
    Ok(())
}

/// Listen for connections (TCP only)
pub fn listen(fd: SocketFd, backlog: u32) -> Result<(), &'static str> {
    let mut sockets = SOCKETS.lock();
    let sock = sockets.get_mut(&fd).ok_or("Invalid socket")?;
    
    if sock.socket_type != SocketType::Stream {
        return Err("Not a stream socket");
    }
    
    if sock.state != SocketState::Bound {
        return Err("Socket not bound");
    }
    
    sock.backlog = backlog;
    sock.state = SocketState::Listening;
    
    Ok(())
}

/// Accept a connection (TCP only)
pub fn accept(fd: SocketFd) -> Result<(SocketFd, SocketAddrV4), &'static str> {
    let mut sockets = SOCKETS.lock();
    let sock = sockets.get_mut(&fd).ok_or("Invalid socket")?;
    
    if sock.state != SocketState::Listening {
        return Err("Socket not listening");
    }
    
    // Would block until connection available
    // For now, return error
    Err("No pending connections")
}

/// Connect to remote address
pub fn connect(fd: SocketFd, addr: SocketAddrV4) -> Result<(), &'static str> {
    let mut sockets = SOCKETS.lock();
    let sock = sockets.get_mut(&fd).ok_or("Invalid socket")?;
    
    if sock.state == SocketState::Connected {
        return Err("Already connected");
    }
    
    // Bind to ephemeral port if not bound
    if sock.local_addr.is_none() {
        let port = super::tcp::alloc_port();
        sock.local_addr = Some(SocketAddrV4::any(port));
    }
    
    sock.remote_addr = Some(addr);
    sock.state = SocketState::Connecting;
    
    // For TCP, would initiate 3-way handshake here
    // For now, just mark as connected
    sock.state = SocketState::Connected;
    
    Ok(())
}

/// Send data
pub fn send(fd: SocketFd, data: &[u8]) -> Result<usize, &'static str> {
    let mut sockets = SOCKETS.lock();
    let sock = sockets.get_mut(&fd).ok_or("Invalid socket")?;
    
    if sock.state != SocketState::Connected {
        return Err("Socket not connected");
    }
    
    // Add to send buffer
    sock.send_buffer.extend_from_slice(data);
    
    // Would actually send data here
    
    Ok(data.len())
}

/// Receive data
pub fn recv(fd: SocketFd, buf: &mut [u8]) -> Result<usize, &'static str> {
    let mut sockets = SOCKETS.lock();
    let sock = sockets.get_mut(&fd).ok_or("Invalid socket")?;
    
    if sock.state != SocketState::Connected {
        return Err("Socket not connected");
    }
    
    // Copy from receive buffer
    let len = core::cmp::min(buf.len(), sock.recv_buffer.len());
    buf[..len].copy_from_slice(&sock.recv_buffer[..len]);
    sock.recv_buffer.drain(..len);
    
    Ok(len)
}

/// Send to specific address (UDP)
pub fn sendto(fd: SocketFd, data: &[u8], addr: SocketAddrV4) -> Result<usize, &'static str> {
    let mut sockets = SOCKETS.lock();
    let sock = sockets.get_mut(&fd).ok_or("Invalid socket")?;
    
    if sock.socket_type != SocketType::Dgram {
        return Err("Not a datagram socket");
    }
    
    // Bind if not bound
    if sock.local_addr.is_none() {
        let port = super::tcp::alloc_port();
        sock.local_addr = Some(SocketAddrV4::any(port));
        sock.state = SocketState::Bound;
    }
    
    // Would send UDP packet here
    let _ = addr;
    
    Ok(data.len())
}

/// Receive from (UDP)
pub fn recvfrom(fd: SocketFd, buf: &mut [u8]) -> Result<(usize, SocketAddrV4), &'static str> {
    let mut sockets = SOCKETS.lock();
    let sock = sockets.get_mut(&fd).ok_or("Invalid socket")?;
    
    if sock.socket_type != SocketType::Dgram {
        return Err("Not a datagram socket");
    }
    
    // Would block until data available
    let len = core::cmp::min(buf.len(), sock.recv_buffer.len());
    buf[..len].copy_from_slice(&sock.recv_buffer[..len]);
    sock.recv_buffer.drain(..len);
    
    // Return sender address (placeholder)
    Ok((len, SocketAddrV4::any(0)))
}

/// Close socket
pub fn close(fd: SocketFd) -> Result<(), &'static str> {
    let mut sockets = SOCKETS.lock();
    sockets.remove(&fd).ok_or("Invalid socket")?;
    Ok(())
}

/// Get socket info
pub fn get_socket(fd: SocketFd) -> Option<(SocketDomain, SocketType, SocketState)> {
    let sockets = SOCKETS.lock();
    sockets.get(&fd).map(|s| (s.domain, s.socket_type, s.state))
}

/// List all sockets
pub fn list_sockets() -> Vec<SocketFd> {
    SOCKETS.lock().keys().cloned().collect()
}

