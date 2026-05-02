//! Placeholder Networking Module for NebulaOS.

use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use smoltcp::phy::{Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;
use smoltcp::iface::{Config, Interface, SocketSet, SocketHandle};
use smoltcp::socket::{dhcpv4, tcp, dns};
use smoltcp::wire::{EthernetAddress, IpCidr, IpAddress, Ipv4Address};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectionType {
    None = 0,
    Ethernet = 1,
    Wifi = 2,
}

/// Represents the simulated network signal strength (0-100%).
/// Non-functional for now, but can be updated by a future network driver.
pub static NETWORK_SIGNAL_STRENGTH: AtomicU8 = AtomicU8::new(0); // Default to 0%
pub static CONNECTION_TYPE: AtomicU8 = AtomicU8::new(ConnectionType::None as u8);
pub static MAC_ADDRESS: spin::Mutex<MacAddress> = spin::Mutex::new(MacAddress([0; 6]));
pub static RX_PACKETS: AtomicUsize = AtomicUsize::new(0);
pub static TX_PACKETS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    pub fn broadcast() -> Self {
        Self([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
    }
}

#[repr(C, packed)]
pub struct EthernetHeader {
    pub dest: MacAddress,
    pub src: MacAddress,
    pub ethertype: u16,
}

/// Glue structure to interface RTL8139 hardware with smoltcp.
pub struct Rtl8139Device;

impl Device for Rtl8139Device {
    type RxToken<'a> = Rtl8139RxToken where Self: 'a;
    type TxToken<'a> = Rtl8139TxToken;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let port = crate::drivers::ethernet::RTL8139_IO.load(Ordering::Relaxed);
        if port == 0 { return None; }

        unsafe {
            // Check if buffer is empty (bit 0 of REG_CMD is 1 if empty)
            if (crate::kernel::io::inb(port + 0x37) & 0x01) != 0 {
                return None;
            }

            let rx_buf_lock = crate::drivers::ethernet::RX_BUFFER.lock();
            let rx_buf = rx_buf_lock.as_ref()?;
            let mut offset = crate::drivers::ethernet::RX_OFFSET.load(Ordering::Relaxed);

            // RTL8139 Packet Header: [Status (2 bytes)][Length (2 bytes)]
            let header_status = u16::from_le_bytes([rx_buf[offset], rx_buf[offset + 1]]);
            let header_len = u16::from_le_bytes([rx_buf[offset + 2], rx_buf[offset + 3]]);

            // Check for ROK (Receive OK) bit
            if (header_status & 0x01) == 0 {
                return None;
            }

            // Packet length includes the 4-byte header and 4-byte CRC at the end
            let data_len = (header_len as usize) - 4;
            let mut packet_data = alloc::vec![0u8; data_len];
            
            // Copy packet data (skipping the 4-byte header)
            for i in 0..data_len {
                packet_data[i] = rx_buf[(offset + 4 + i) % 8192];
            }

            // Update internal offset (4 bytes header + length, aligned to 4 bytes)
            offset = (offset + header_len as usize + 4 + 3) & !3;
            offset %= 8192;
            crate::drivers::ethernet::RX_OFFSET.store(offset, Ordering::Relaxed);

            // Update hardware: REG_CAPR expects offset - 16
            crate::kernel::io::outw(port + 0x38, (offset as u16).wrapping_sub(16));

            RX_PACKETS.fetch_add(1, Ordering::Relaxed);
            Some((Rtl8139RxToken { data: packet_data }, Rtl8139TxToken))
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(Rtl8139TxToken)
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1500;
        caps.medium = Medium::Ethernet;
        caps
    }
}

pub struct Rtl8139RxToken {
    pub data: Vec<u8>,
}
impl smoltcp::phy::RxToken for Rtl8139RxToken {
    fn consume<R, F>(self, f: F) -> R where F: FnOnce(&[u8]) -> R {
        f(&self.data)
    }
}

pub struct Rtl8139TxToken;
impl smoltcp::phy::TxToken for Rtl8139TxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R where F: FnOnce(&mut [u8]) -> R {
        let mut buffer = [0u8; 1500];
        let result = f(&mut buffer[..len]);
        
        // Hardware Transmit Logic
        let port = crate::drivers::ethernet::RTL8139_IO.load(Ordering::Relaxed);
        if port != 0 {
            unsafe {
                // Set physical address and length, then trigger transfer
                crate::kernel::io::outl(port + 0x20, buffer.as_ptr() as u32);
                crate::kernel::io::outl(port + 0x10, len as u32);
            }
            TX_PACKETS.fetch_add(1, Ordering::Relaxed);
        }
        result
    }
}

// Storage for the actual socket objects. These need to be 'static.
// We'll use a fixed-size array of Options to allow for dynamic initialization.
const MAX_SOCKETS: usize = 4; 
static mut SOCKET_STORAGE: [Option<Box<dyn smoltcp::socket::Socket + 'static>>; MAX_SOCKETS] = [None, None, None, None];

pub static DNS_HANDLE: spin::Mutex<Option<SocketHandle>> = spin::Mutex::new(None);
pub static HTTP_HANDLE: spin::Mutex<Option<SocketHandle>> = spin::Mutex::new(None);

// The SocketSet itself, which will hold references to the sockets in SOCKET_STORAGE.
// This also needs to be wrapped in a Mutex and Option because it's initialized later.
pub static SOCKET_SET: spin::Mutex<Option<SocketSet<'static>>> = spin::Mutex::new(None);
pub static INTERFACE: spin::Mutex<Option<Interface>> = spin::Mutex::new(None);

pub fn init() {
    let mac = MAC_ADDRESS.lock();
    let eth_addr = EthernetAddress([mac.0[0], mac.0[1], mac.0[2], mac.0[3], mac.0[4], mac.0[5]]);
    
    let mut device = Rtl8139Device;
    let config = Config::new(eth_addr.into());
    
    // --- Initialize Sockets and SocketSet ---
    let mut sockets_refs: Vec<&'static mut dyn smoltcp::socket::Socket> = Vec::new();

    // DHCP Socket
    unsafe {
        SOCKET_STORAGE[0] = Some(Box::new(dhcpv4::Socket::new()));
        if let Some(ref mut s) = SOCKET_STORAGE[0] {
            sockets_refs.push(s.as_mut());
        }
    }

    // Create the SocketSet
    let mut socket_set = SocketSet::new(sockets_refs);

    // Initialize DNS Socket
    let dns_socket = dns::Socket::new(&[], alloc::vec![]);
    let dns_handle = socket_set.add(dns_socket);
    *DNS_HANDLE.lock() = Some(dns_handle);

    // Initialize TCP Socket (for HTTP)
    let tcp_rx_buffer = tcp::SocketBuffer::new(alloc::vec![0; 4096]);
    let tcp_tx_buffer = tcp::SocketBuffer::new(alloc::vec![0; 4096]);
    let tcp_socket = tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer);
    let tcp_handle = socket_set.add(tcp_socket);
    *HTTP_HANDLE.lock() = Some(tcp_handle);

    *SOCKET_SET.lock() = Some(socket_set);

    let mut iface = Interface::new(config, &mut device, Instant::from_millis(0));    

    *INTERFACE.lock() = Some(iface);
    crate::serial_println!("[NET] Networking module initialized.");
}

pub fn poll(timestamp: Instant) {
    let mut interface_guard = INTERFACE.lock();
    let mut socket_set_guard = SOCKET_SET.lock();
    
    if let (Some(ref mut interface), Some(ref mut sockets)) = (&mut *interface_guard, &mut *socket_set_guard) {
        let mut device = Rtl8139Device; // Create a new instance for each poll
        
        // Poll the interface
        let _ = interface.poll(timestamp, &mut device, sockets);

        // Poll the DHCP socket
        // Iterate through sockets to find the DHCP one
        for (_handle, socket) in sockets.iter_mut() {
            if let Some(dhcp_socket) = socket.as_dhcpv4() {
                match dhcp_socket.poll(interface, &mut device, timestamp) {
                    Ok(event) => {
                        match event {
                            dhcpv4::Event::Configured(config) => {
                                crate::serial_println!("[NET] DHCP Configured: {:?}", config);
                                interface.update_ip_addrs(|addrs| {
                                    addrs.clear();
                                    addrs.push(config.address).unwrap();
                                });
                                if let Some(gateway) = config.router {
                                    interface.update_routes(|routes| {
                                        routes.add_default_ipv4_route(gateway).unwrap();
                                    });
                                }
                                set_connection(ConnectionType::Ethernet, 100); // Assuming Ethernet for now
                            }
                            dhcpv4::Event::Deconfigured => {
                                crate::serial_println!("[NET] DHCP Deconfigured.");
                                interface.update_ip_addrs(|addrs| addrs.clear());
                                interface.update_routes(|routes| routes.clear());
                                set_connection(ConnectionType::None, 0);
                            }
                        }
                    }
                    Err(_e) => {
                        // DHCP errors are common during initial negotiation,
                        // so we might not want to spam the serial.
                        // crate::serial_println!("[NET] DHCP Error: {:?}", _e);
                    }
                }
            }
        }
    }
}

pub fn set_connection(conn_type: ConnectionType, strength: u8) {
    CONNECTION_TYPE.store(conn_type as u8, Ordering::Relaxed);
    NETWORK_SIGNAL_STRENGTH.store(strength, Ordering::Relaxed);
}