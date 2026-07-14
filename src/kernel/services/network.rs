// Networking Service for NebulaOS
// Basic TCP/IP stack implementation

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, Ordering};

/// Network socket types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketType {
    TCP,
    UDP,
    Raw,
}

/// Socket state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SocketState {
    Closed,
    Listening,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    Closing,
    TimeWait,
    CloseWait,
    LastAck,
}

/// Network socket
pub struct Socket {
    pub socket_id: u32,
    pub socket_type: SocketType,
    pub state: SocketState,
    pub local_addr: (u32, u16),  // (IP address, port)
    pub remote_addr: (u32, u16), // (IP address, port)
    pub send_buffer: Vec<u8>,
    pub recv_buffer: Vec<u8>,
}

impl Socket {
    pub fn new(socket_type: SocketType) -> Self {
        static NEXT_SOCKET_ID: AtomicU32 = AtomicU32::new(1);
        let socket_id = NEXT_SOCKET_ID.fetch_add(1, Ordering::SeqCst);
        
        Socket {
            socket_id,
            socket_type,
            state: SocketState::Closed,
            local_addr: (0, 0),
            remote_addr: (0, 0),
            send_buffer: Vec::new(),
            recv_buffer: Vec::new(),
        }
    }
    
    pub fn bind(&mut self, addr: (u32, u16)) -> Result<(), &'static str> {
        self.local_addr = addr;
        self.state = SocketState::Closed;
        Ok(())
    }
    
    pub fn listen(&mut self, backlog: i32) -> Result<(), &'static str> {
        if self.socket_type != SocketType::TCP {
            return Err("Only TCP sockets can listen");
        }
        self.state = SocketState::Listening;
        Ok(())
    }
    
    pub fn connect(&mut self, addr: (u32, u16)) -> Result<(), &'static str> {
        self.remote_addr = addr;
        self.state = SocketState::SynSent;
        // In a real implementation, we would send SYN packet here
        Ok(())
    }
    
    pub fn send(&mut self, data: &[u8]) -> Result<usize, &'static str> {
        self.send_buffer.extend_from_slice(data);
        Ok(data.len())
    }
    
    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        let bytes_to_copy = buffer.len().min(self.recv_buffer.len());
        buffer[..bytes_to_copy].copy_from_slice(&self.recv_buffer[..bytes_to_copy]);
        self.recv_buffer.drain(..bytes_to_copy);
        Ok(bytes_to_copy)
    }
    
    pub fn close(&mut self) -> Result<(), &'static str> {
        self.state = SocketState::Closed;
        Ok(())
    }
}

/// Network service
pub struct NetworkService {
    sockets: BTreeMap<u32, Socket>,
    network_interfaces: Vec<NetworkInterface>,
}

impl NetworkService {
    pub fn new() -> Self {
        NetworkService {
            sockets: BTreeMap::new(),
            network_interfaces: Vec::new(),
        }
    }
    
    pub fn create_socket(&mut self, socket_type: SocketType) -> Result<u32, &'static str> {
        let socket = Socket::new(socket_type);
        let socket_id = socket.socket_id;
        self.sockets.insert(socket_id, socket);
        Ok(socket_id)
    }
    
    pub fn get_socket(&self, socket_id: u32) -> Option<&Socket> {
        self.sockets.get(&socket_id)
    }
    
    pub fn get_socket_mut(&mut self, socket_id: u32) -> Option<&mut Socket> {
        self.sockets.get_mut(&socket_id)
    }
    
    pub fn close_socket(&mut self, socket_id: u32) -> Result<(), &'static str> {
        if let Some(socket) = self.sockets.remove(&socket_id) {
            drop(socket);
            Ok(())
        } else {
            Err("Socket not found")
        }
    }
    
    pub fn add_network_interface(&mut self, interface: NetworkInterface) {
        self.network_interfaces.push(interface);
    }
    
    pub fn get_interfaces(&self) -> &[NetworkInterface] {
        &self.network_interfaces
    }
}

/// Network interface
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub mac_address: [u8; 6],
    pub ip_address: u32,
    pub netmask: u32,
    pub gateway: u32,
    pub mtu: u16,
    pub is_up: bool,
}

impl NetworkInterface {
    pub fn new(name: &str, mac_address: [u8; 6]) -> Self {
        NetworkInterface {
            name: name.to_string(),
            mac_address,
            ip_address: 0,
            netmask: 0,
            gateway: 0,
            mtu: 1500,
            is_up: false,
        }
    }
    
    pub fn configure(&mut self, ip_address: u32, netmask: u32, gateway: u32) {
        self.ip_address = ip_address;
        self.netmask = netmask;
        self.gateway = gateway;
        self.is_up = true;
    }
    
    pub fn ip_to_string(&self) -> String {
        format!("{}.{}.{}.{}",
            (self.ip_address >> 24) & 0xFF,
            (self.ip_address >> 16) & 0xFF,
            (self.ip_address >> 8) & 0xFF,
            self.ip_address & 0xFF
        )
    }
}

/// Global network service instance
pub static NETWORK_SERVICE: spin::Mutex<NetworkService> = spin::Mutex::new(NetworkService::new());

/// Initialize the network service
pub fn init() {
    let mut service = NETWORK_SERVICE.lock();
    
    // Add loopback interface
    let mut lo = NetworkInterface::new("lo", [0, 0, 0, 0, 0, 0]);
    lo.configure(0x7F000001, 0xFF000000, 0); // 127.0.0.1/8
    service.add_network_interface(lo);
}