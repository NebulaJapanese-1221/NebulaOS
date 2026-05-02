//! Placeholder Networking Module for NebulaOS.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use smoltcp::phy::{Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::socket::{dhcpv4, tcp, dns};
use smoltcp::socket::Socket; // Import the Socket enum
use smoltcp::wire::{EthernetAddress, IpCidr};
use rustls::RootCertStore;

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

/// I/O Shims to bridge memory slices with no_std environments.
pub mod io {
    pub struct SliceReader<'a> {
        pub data: &'a [u8],
        pub pos: usize,
    }

    impl<'a> SliceReader<'a> {
        pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
            let amt = core::cmp::min(buf.len(), self.data.len() - self.pos);
            if amt == 0 { return Ok(0); }
            buf[..amt].copy_from_slice(&self.data[self.pos..self.pos + amt]);
            self.pos += amt;
            Ok(amt)
        }
    }

    pub struct SliceWriter<'a> {
        pub data: &'a mut [u8],
        pub pos: usize,
    }

    impl<'a> SliceWriter<'a> {
        pub fn write(&mut self, buf: &[u8]) -> Result<usize, ()> {
            let amt = core::cmp::min(buf.len(), self.data.len() - self.pos);
            if amt == 0 { return Ok(0); }
            self.data[self.pos..self.pos + amt].copy_from_slice(&buf[..amt]);
            self.pos += amt;
            Ok(amt)
        }
    }
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
        f(&self.data[..])
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

pub static DNS_HANDLE: spin::Mutex<Option<smoltcp::iface::SocketHandle>> = spin::Mutex::new(None);
pub static HTTP_HANDLE: spin::Mutex<Option<smoltcp::iface::SocketHandle>> = spin::Mutex::new(None);

// The SocketSet itself, which will hold references to the sockets in SOCKET_STORAGE.
// This also needs to be wrapped in a Mutex and Option because it's initialized later.
pub static SOCKET_SET: spin::Mutex<Option<SocketSet<'static>>> = spin::Mutex::new(None);
pub static INTERFACE: spin::Mutex<Option<Interface>> = spin::Mutex::new(None);
pub static ROOT_CERT_STORE: spin::Mutex<Option<RootCertStore>> = spin::Mutex::new(None);

/// Initializes the Root Certificate store with trusted Certificate Authorities.
pub fn init_ca_store() {
    let store = RootCertStore::empty();
    
    // Example: Embedding a well-known Root CA (like ISRG Root X1 for Let's Encrypt)
    // In a full implementation, you would bundle common Root CAs in DER format.
    // let root_ca_der = include_bytes!("../certs/isrgrootx1.der");
    // if let Ok(cert) = rustls_pki_types::CertificateDer::try_from(root_ca_der.as_slice()) {
    //     store.add(cert).ok();
    // }

    *ROOT_CERT_STORE.lock() = Some(store);
    crate::serial_println!("[NET] Root CA store initialized.");
}

pub fn init() {
    let mac = MAC_ADDRESS.lock();
    let eth_addr = EthernetAddress([mac.0[0], mac.0[1], mac.0[2], mac.0[3], mac.0[4], mac.0[5]]);
    
    let mut device = Rtl8139Device;
    let config = Config::new(eth_addr.into());
    
    // --- Initialize Sockets and SocketSet ---
    let mut socket_set = SocketSet::new(Vec::new());

    // DHCP Socket
    // DHCP socket needs to be added to SOCKET_STORAGE and then to the SocketSet
    let dhcp_socket = Socket::Dhcpv4(dhcpv4::Socket::new());
    let _dhcp_handle = socket_set.add(dhcp_socket);
    // Store the handle if needed, but for DHCP we often just iterate
    // through sockets to find it.


    // Initialize DNS Socket
    let dns_socket = Socket::Dns(dns::Socket::new(&[], Vec::new()));
    let dns_handle = socket_set.add(dns_socket);
    *DNS_HANDLE.lock() = Some(dns_handle);

    // Initialize TCP Socket (for HTTP)
    let tcp_rx_buffer = tcp::SocketBuffer::new(Vec::new());
    let tcp_tx_buffer = tcp::SocketBuffer::new(Vec::new());
    let tcp_socket = Socket::Tcp(tcp::Socket::new(tcp_rx_buffer, tcp_tx_buffer));
    let tcp_handle = socket_set.add(tcp_socket);
    *HTTP_HANDLE.lock() = Some(tcp_handle);

    *SOCKET_SET.lock() = Some(socket_set);

    let mut iface = Interface::new(config, &mut device, Instant::from_millis(0));    

    *INTERFACE.lock() = Some(iface);
    init_ca_store();

    crate::serial_println!("[NET] Networking module initialized.");
}

pub fn poll(timestamp: Instant) {
    let mut interface_guard = INTERFACE.lock();
    let mut socket_set_guard = SOCKET_SET.lock();
    
    if let (Some(interface), Some(sockets)) = (&mut *interface_guard, &mut *socket_set_guard) {
        let mut device = Rtl8139Device; // Create a new instance for each poll
        
        // Poll the interface
        let _ = interface.poll(timestamp, &mut device, sockets);

        // Poll the DHCP socket
        // Iterate through sockets to find the DHCP one
        for (_handle, socket) in sockets.iter_mut() {
            if let Socket::Dhcpv4(dhcp_socket) = socket {
                match dhcp_socket.poll() {
                    Some(event) => {
                        match event {
                            dhcpv4::Event::Configured(config) => {
                                crate::serial_println!("[NET] DHCP Configured: {:?}", config);
                                interface.update_ip_addrs(|addrs| {
                                    addrs.clear();
                                    addrs.push(smoltcp::wire::IpCidr::Ipv4(config.address)).unwrap();
                                });
                                if let Some(gateway) = config.router {
                                    interface.routes_mut().add_default_ipv4_route(gateway).unwrap();
                                }
                                set_connection(ConnectionType::Ethernet, 100); // Assuming Ethernet for now
                            }
                            dhcpv4::Event::Deconfigured => {
                                crate::serial_println!("[NET] DHCP Deconfigured.");
                                interface.update_ip_addrs(|addrs| addrs.clear());
                                interface.routes_mut().remove_default_ipv4_route();
                                set_connection(ConnectionType::None, 0);
                            }
                        }
                    }
                    None => {}
                }
            }
        }
    }
}

pub fn set_connection(conn_type: ConnectionType, strength: u8) {
    CONNECTION_TYPE.store(conn_type as u8, Ordering::Relaxed);
    NETWORK_SIGNAL_STRENGTH.store(strength, Ordering::Relaxed);
}