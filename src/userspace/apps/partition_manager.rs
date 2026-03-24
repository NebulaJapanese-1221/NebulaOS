use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::format;
use crate::drivers::ata::AtaDrive;
use core::convert::TryInto;
use nebulafs::vdev::Vdev;
use alloc::sync::Arc;
use nebulafs::dmu::ObjsetPhys;

#[derive(Clone)]
struct PartitionEntry {
    id: usize,
    status: u8,
    type_code: u8,
    lba_start: u32,
    sector_count: u32,
}

#[derive(Clone)]
pub struct PartitionManager {
    partitions: Vec<PartitionEntry>,
    drive_status: String,
    fs_type: String,
    files: Vec<String>,
}

impl PartitionManager {
    pub fn new() -> Self {
        let mut pm = Self {
            partitions: Vec::new(),
            drive_status: String::from("Reading Primary Master..."),
            fs_type: String::from("Unknown"),
            files: Vec::new(),
        };
        pm.refresh();
        pm
    }

    fn refresh(&mut self) {
        self.partitions.clear();
        self.files.clear();
        let drive = AtaDrive::new(true, true); // Primary Master
        // Read MBR (LBA 0)
        let data = drive.read_sectors(0, 1);

        if data.len() != 512 {
            self.drive_status = String::from("Error: Read Failed");
            return;
        }

        // Check Signature (Offset 510 = 0x55, 511 = 0xAA)
        if data[510] != 0x55 || data[511] != 0xAA {
            self.drive_status = String::from("Error: Invalid MBR Signature");
            return;
        }

        self.drive_status = String::from("MBR Detected. Primary Master.");

        // Parse 4 partition entries starting at offset 446 (0x1BE)
        for i in 0..4 {
            let offset = 446 + (i * 16);
            let status = data[offset];
            let type_code = data[offset + 4];
            
            // LBA Start (u32 little endian at offset 8)
            let lba_start = u32::from_le_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11]
            ]);

            // Sector Count (u32 little endian at offset 12)
            let sector_count = u32::from_le_bytes([
                data[offset + 12],
                data[offset + 13],
                data[offset + 14],
                data[offset + 15]
            ]);

            // Only list partitions with a non-zero size
            if sector_count > 0 {
                self.partitions.push(PartitionEntry {
                    id: i + 1,
                    status,
                    type_code,
                    lba_start,
                    sector_count,
                });
            }
        }
        
        if self.partitions.is_empty() {
             self.drive_status = String::from("Disk is empty (No partitions found).");
        }

        // Attempt to mount NebulaFS
        let mut root_vdev = Vdev::new_leaf(0, 0, "ata0", 0, 9);
        root_vdev.backend = Some(Arc::new(drive)); // Inject ATA driver as backend
        
        if let Some(spa) = nebulafs::spa::Spa::find(root_vdev) {
            self.fs_type = String::from("NebulaFS (Detected)");
            
            // Try to read root directory
            // 1. Get Root Block Pointer from Uberblock
            let root_bp = spa.uberblock.rootbp;
            
            // 2. Read the Object Set (FileSystem) pointed to by RootBP
            // In this simple version, we assume RootBP points to a single block containing the ObjsetPhys
            let os_data = spa.root_vdev.read_block(root_bp.offset, root_bp.asize as usize); 
            
            if os_data.len() >= core::mem::size_of::<ObjsetPhys>() {
                let os = unsafe { &*(os_data.as_ptr() as *const ObjsetPhys) };
                
                // 3. Get the Root Directory Dnode.
                // In ZFS, the "Master Node" is usually object ID 1.
                // The Master Node is a ZAP containing keys like "ROOT" -> Object ID of root directory.
                // For NebulaFS v0.0.3, let's assume Object ID 2 IS the root directory for simplicity.
                if let Some(root_dnode) = os.get_dnode(&spa.root_vdev, 2) {
                    // 4. Read Directory Contents (ZAP)
                    // Assuming the directory is small enough to fit in the first block
                    // and is a "MicroZAP" (simple linear array of entries).
                    if let Some(dir_data) = root_dnode.read_data(&spa.root_vdev, 0, root_dnode.datablksz as usize) {
                        let entries = nebulafs::zap::parse_directory(&dir_data);
                        for entry in entries {
                            let mut name = entry.name;
                            if entry.type_ == 4 { name.push('/'); } // Directory marker
                            self.files.push(name);
                        }
                    }
                }
            }
            
            if self.files.is_empty() {
                 self.files.push(String::from("<Empty Pool>"));
            }
        } else {
            self.fs_type = String::from("None / Raw");
        }
    }
}

impl App for PartitionManager {
    fn draw(&self, fb: &mut framebuffer::Framebuffer, win: &Window) {
        // Draw background
        gui::draw_rect(fb, win.x, win.y + 20, win.width, win.height - 20, 0x00_20_20_20, None);

        let font_height = 16;
        let x = win.x + 10;
        let mut y = win.y + 30;

        // Status
        font::draw_string(fb, x, y, &format!("{} | FS: {}", self.drive_status, self.fs_type), 0x00_FF_FF_00, None);
        y += font_height as isize + 10;

        // Table Header
        let header_color = 0x00_FF_FF_FF;
        font::draw_string(fb, x, y, "#", header_color, None);
        font::draw_string(fb, x + 30, y, "Boot", header_color, None);
        font::draw_string(fb, x + 80, y, "Type", header_color, None);
        font::draw_string(fb, x + 130, y, "Start LBA", header_color, None);
        font::draw_string(fb, x + 230, y, "Size (MB)", header_color, None);
        
        y += font_height as isize + 5;
        gui::draw_rect(fb, x, y, win.width - 20, 1, 0x00_80_80_80, None);
        y += 5;

        // Entries
        for part in &self.partitions {
            let row_color = 0x00_CC_CC_CC;
            let boot_flag = if part.status == 0x80 { "*" } else { "" };
            // Size in MB = sectors * 512 / 1024 / 1024 = sectors / 2048
            let size_mb = part.sector_count / 2048;

            font::draw_string(fb, x, y, format!("{}", part.id).as_str(), row_color, None);
            font::draw_string(fb, x + 30, y, boot_flag, 0x00_00_FF_00, None);
            font::draw_string(fb, x + 80, y, format!("0x{:02X}", part.type_code).as_str(), row_color, None);
            font::draw_string(fb, x + 130, y, format!("{}", part.lba_start).as_str(), row_color, None);
            font::draw_string(fb, x + 230, y, format!("{} MB", size_mb).as_str(), row_color, None);

            y += font_height as isize + 5;
        }

        // File Browser Section
        y += 10;
        gui::draw_rect(fb, x, y, win.width - 20, 1, 0x00_80_80_80, None); // Separator
        y += 5;
        font::draw_string(fb, x, y, "File Browser (Root)", 0x00_FF_FF_FF, None);
        y += font_height as isize + 5;

        // File List Background
        gui::draw_rect(fb, x, y, win.width - 20, win.height.saturating_sub((y - win.y) as usize) - 10, 0x00_10_10_10, None);
        y += 5;
        let file_x = x + 5;

        for file in &self.files {
            let color = if file.ends_with('/') { 0x00_FFFF00 } else { 0x00_CCCCCC };
            font::draw_string(fb, file_x, y, file, color, None);
            y += font_height as isize + 2;
        }
    }

    fn handle_event(&mut self, _event: &AppEvent) {}

    fn box_clone(&self) -> Box<dyn App> {
        Box::new(self.clone())
    }
}