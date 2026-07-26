use alloc::string::{String, ToString};
use alloc::vec::Vec;
use self::block::BlockDeviceManager;

pub mod block {
    use alloc::vec::Vec;
    use alloc::boxed::Box;

    pub trait BlockDevice: Send + Sync + core::fmt::Debug {
        fn read_blocks(&self, start_block: u64, block_count: u64, buffer: &mut [u8]) -> Result<(), &'static str>;
        fn write_blocks(&self, start_block: u64, block_count: u64, buffer: &[u8]) -> Result<(), &'static str>;
        fn flush(&self) -> Result<(), &'static str>;
        fn get_info(&self) -> BlockDeviceInfo;
    }

    #[derive(Debug, Clone, Copy)]
    pub struct BlockDeviceInfo {
        pub block_size: u64,
        pub total_blocks: u64,
        pub device_name: &'static str,
    }

    #[derive(Debug)]
    pub struct BlockDeviceManager {
        devices: Vec<Box<dyn BlockDevice>>,
    }

    impl BlockDeviceManager {
        pub fn new() -> Self {
            BlockDeviceManager { devices: Vec::new() }
        }
        pub fn register_device(&mut self, device: Box<dyn BlockDevice>) {
            self.devices.push(device);
        }
        pub fn get_device(&self, index: usize) -> Option<&dyn BlockDevice> {
            self.devices.get(index).map(|d| d.as_ref())
        }
        pub fn device_count(&self) -> usize {
            self.devices.len()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VDevType {
    Disk,
    File,
    Mirror,
    RaidZ,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VDevState {
    Unknown,
    Online,
    Degraded,
    Faulted,
    Offline,
    Removed,
}

#[derive(Clone, Debug)]
pub struct VDevStats {
    pub reads: u64,
    pub writes: u64,
    pub read_errors: u64,
    pub write_errors: u64,
    pub checksum_errors: u64,
    pub io_in_progress: u64,
}

impl VDevStats {
    pub fn new() -> Self {
        VDevStats { reads: 0, writes: 0, read_errors: 0, write_errors: 0, checksum_errors: 0, io_in_progress: 0 }
    }
}

#[derive(Debug, Clone)]
pub struct VDev {
    pub vdev_id: u64,
    pub vdev_type: VDevType,
    pub state: VDevState,
    pub size: u64,
    pub stats: VDevStats,
    pub children: Vec<VDev>,
    pub path: Option<String>,
    pub fd: Option<i32>,
    pub block_device: Option<u8>,
    pub device_manager: Option<&'static BlockDeviceManager>,
}

impl VDev {
    pub fn new(vdev_type: VDevType, size: u64) -> Self {
        VDev {
            vdev_id: 0, vdev_type, state: VDevState::Unknown, size,
            stats: VDevStats::new(), children: Vec::new(), path: None,
            fd: None, block_device: None, device_manager: None,
        }
    }

    pub fn new_disk(device_manager: &'static BlockDeviceManager, device_index: u8, size: u64) -> Self {
        let mut vdev = VDev::new(VDevType::Disk, size);
        vdev.block_device = Some(device_index);
        vdev.device_manager = Some(device_manager);
        vdev
    }

    pub fn new_file(path: &str, size: u64) -> Self {
        let mut vdev = VDev::new(VDevType::File, size);
        vdev.path = Some(path.to_string());
        vdev.fd = Some(-1);
        vdev
    }

    pub fn new_mirror(children: Vec<VDev>) -> Self {
        let min_size = children.iter().map(|child| child.size).min().unwrap_or(0);
        VDev { vdev_id: 0, vdev_type: VDevType::Mirror, state: VDevState::Unknown, size: min_size,
            stats: VDevStats::new(), children, path: None, fd: None, block_device: None, device_manager: None }
    }

    pub fn open(&mut self) -> Result<(), &'static str> {
        match self.vdev_type {
            VDevType::Disk => { self.state = VDevState::Online; Ok(()) }
            VDevType::File => {
                if self.path.is_some() { self.state = VDevState::Online; Ok(()) }
                else { Err("No path specified") }
            }
            VDevType::Mirror | VDevType::RaidZ => {
                for child in &mut self.children { child.open()?; }
                self.state = VDevState::Online; Ok(())
            }
            VDevType::Missing => { self.state = VDevState::Faulted; Ok(()) }
        }
    }

    pub fn close(&mut self) -> Result<(), &'static str> {
        match self.vdev_type {
            VDevType::Disk | VDevType::File => { self.state = VDevState::Offline; Ok(()) }
            VDevType::Mirror | VDevType::RaidZ => {
                for child in &mut self.children { child.close()?; }
                self.state = VDevState::Offline; Ok(())
            }
            VDevType::Missing => { self.state = VDevState::Removed; Ok(()) }
        }
    }

    pub fn read(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, &'static str> {
        self.stats.reads += 1;
        self.stats.io_in_progress += 1;
        let result = match self.vdev_type {
            VDevType::Disk => self.read_disk(offset, buffer),
            VDevType::File => self.read_file_vdev(offset, buffer),
            VDevType::Mirror => self.read_mirror(offset, buffer),
            VDevType::RaidZ => self.read_raidz(offset, buffer),
            VDevType::Missing => Err("Cannot read from missing vdev"),
        };
        self.stats.io_in_progress -= 1;
        result
    }

    pub fn write(&mut self, offset: u64, data: &[u8]) -> Result<usize, &'static str> {
        self.stats.writes += 1;
        self.stats.io_in_progress += 1;
        let result = match self.vdev_type {
            VDevType::Disk => self.write_disk(offset, data),
            VDevType::File => self.write_file_vdev(offset, data),
            VDevType::Mirror => self.write_mirror(offset, data),
            VDevType::RaidZ => self.write_raidz(offset, data),
            VDevType::Missing => Err("Cannot write to missing vdev"),
        };
        self.stats.io_in_progress -= 1;
        result
    }

    fn read_disk(&self, offset: u64, buffer: &mut [u8]) -> Result<usize, &'static str> {
        let bytes_to_read = buffer.len().min((self.size.saturating_sub(offset)) as usize);
        for (i, byte) in buffer.iter_mut().enumerate().take(bytes_to_read) {
            *byte = ((offset + i as u64) % 256) as u8;
        }
        Ok(bytes_to_read)
    }

    fn write_disk(&mut self, offset: u64, data: &[u8]) -> Result<usize, &'static str> {
        Ok(data.len().min((self.size.saturating_sub(offset)) as usize))
    }

    fn read_file_vdev(&self, offset: u64, buffer: &mut [u8]) -> Result<usize, &'static str> {
        let bytes_to_read = buffer.len().min((self.size.saturating_sub(offset)) as usize);
        for (i, byte) in buffer.iter_mut().enumerate().take(bytes_to_read) {
            *byte = ((offset + i as u64) % 256) as u8;
        }
        Ok(bytes_to_read)
    }

    fn write_file_vdev(&mut self, offset: u64, data: &[u8]) -> Result<usize, &'static str> {
        Ok(data.len().min((self.size.saturating_sub(offset)) as usize))
    }

    fn read_mirror(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, &'static str> {
        let mut last_error = "All mirror children failed";
        for child in &mut self.children {
            if child.is_healthy() {
                let mut temp_buffer = Vec::new();
                temp_buffer.resize(buffer.len(), 0u8);
                match child.read(offset, &mut temp_buffer) {
                    Ok(bytes_read) => {
                        buffer[..bytes_read].copy_from_slice(&temp_buffer[..bytes_read]);
                        return Ok(bytes_read);
                    }
                    Err(e) => { last_error = e; continue; }
                }
            }
        }
        Err(last_error)
    }

    fn write_mirror(&mut self, offset: u64, data: &[u8]) -> Result<usize, &'static str> {
        let mut bytes_written = 0;
        let mut success_count = 0;
        for child in &mut self.children {
            if child.is_healthy() {
                match child.write(offset, data) {
                    Ok(bw) => { bytes_written = bw; success_count += 1; }
                    Err(e) => { return Err(e); }
                }
            }
        }
        if success_count > 0 { Ok(bytes_written) } else { Err("No healthy children in mirror") }
    }

    fn read_raidz(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, &'static str> {
        for child in &mut self.children {
            if child.is_healthy() { return child.read(offset, buffer); }
        }
        Err("No healthy children in RAID-Z")
    }

    fn write_raidz(&mut self, offset: u64, data: &[u8]) -> Result<usize, &'static str> {
        let mut bytes_written = 0;
        for child in &mut self.children {
            if child.is_healthy() { bytes_written = child.write(offset, data)?; }
        }
        Ok(bytes_written)
    }

    pub fn redundancy(&self) -> usize {
        match self.vdev_type { VDevType::Mirror => self.children.len(), VDevType::RaidZ => 1, _ => 1 }
    }

    pub fn is_healthy(&self) -> bool {
        self.state == VDevState::Online || self.state == VDevState::Degraded
    }
}
