use crate::drivers::framebuffer;
use crate::userspace::gui::{self, font, Window};
use super::app::{App, AppEvent};
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;
use crate::drivers::ata::AtaDrive;
use nebulafs::vdev::Vdev;
use alloc::sync::Arc;
use crate::userspace::gui::button::Button;

#[derive(Clone)]
struct PartitionEntry {
    id: usize,
    status: u8,
    fs_type: String, // Replaces type_code for display
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
    btn_init_mbr: Option<Button>,
    btn_format: Option<Button>,
    show_confirm: bool,
}

impl PartitionManager {
    pub fn new() -> Self {
        let mut pm = Self {
            partitions: Vec::new(),
            drive_status: String::from("Reading Primary Master..."),
            fs_type: String::from("Unknown"),
            files: Vec::new(),
            btn_init_mbr: None,
            btn_format: None,
            show_confirm: false,
        };
        pm.refresh();
        pm
    }

    fn refresh(&mut self) {
        self.partitions.clear();
        self.files.clear();
        let drive = Arc::new(AtaDrive::new(true, true)); // Primary Master
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
                let fs_type = self.detect_fs(drive.as_ref(), lba_start, type_code);
                
                self.partitions.push(PartitionEntry {
                    id: i + 1,
                    status,
                    fs_type,
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
        root_vdev.backend = Some(drive.clone()); // Inject ATA driver as backend
        
        if let Some(fs) = nebulafs::fs::NebulaFileSystem::mount(root_vdev) {
            self.fs_type = String::from("NebulaFS (Detected)");
            self.files = fs.list_root();
        } else {
            self.fs_type = String::from("None / Raw");
        }
    }

    fn detect_fs(&self, drive: &AtaDrive, lba_start: u32, type_code: u8) -> String {
        match type_code {
            0x07 => {
                let data = drive.read_sectors(lba_start, 1);
                if data.len() >= 8 && &data[3..7] == b"NTFS" {
                    return String::from("NTFS");
                }
                String::from("HPFS/NTFS")
            },
            0x0B | 0x0C => {
                let data = drive.read_sectors(lba_start, 1);
                if data.len() > 90 && &data[82..87] == b"FAT32" {
                    return String::from("FAT32");
                }
                String::from("FAT32 (LBA)")
            },
            0x83 => {
                // Check Ext4 Superblock (1024 bytes offset, so LBA+2)
                let data = drive.read_sectors(lba_start + 2, 1);
                if data.len() >= 60 {
                    // Magic at offset 56 (0x38)
                    let magic = u16::from_le_bytes([data[56], data[57]]);
                    if magic == 0xEF53 {
                        return String::from("Ext4 (Linux)");
                    }
                }
                String::from("Linux (Ext)")
            },
            _ => format!("0x{:02X}", type_code),
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
        font::draw_string(fb, x + 80, y, "Filesystem", header_color, None);
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
            font::draw_string(fb, x + 80, y, part.fs_type.as_str(), row_color, None);
            font::draw_string(fb, x + 130 + 50, y, format!("{}", part.lba_start).as_str(), row_color, None);
            font::draw_string(fb, x + 230 + 50, y, format!("{} MB", size_mb).as_str(), row_color, None);

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

        // Toolbar at bottom
        let toolbar_y = win.y + win.height as isize - 40;
        
        // Because draw() is immutable &self, we can't update the struct's button fields here directly 
        // if we want to store them. However, we can construct them just for drawing, 
        // and recreate identical ones in handle_event for hit testing.
        // Alternatively, we use interior mutability or just simple recalculation.
        
        let btn_init = Button::new(x, toolbar_y, 140, 30, "Init MBR");
        btn_init.draw(fb, 0, 0, None); // Mouse pos 0,0 for draw as we don't have it here easily
        
        let btn_fmt = Button::new(x + 150, toolbar_y, 160, 30, "Format NebulaFS");
        btn_fmt.draw(fb, 0, 0, None);

        if self.show_confirm {
            let dialog_w = 260;
            let dialog_h = 120;
            let dx = win.x + (win.width as isize - dialog_w) / 2;
            let dy = win.y + (win.height as isize - dialog_h) / 2;

            // Draw dialog box with border
            gui::draw_rect(fb, dx, dy, dialog_w as usize, dialog_h as usize, 0x00_28_28_28, None);
            gui::draw_rect(fb, dx, dy, dialog_w as usize, 1, 0x00_FF_FF_FF, None); // Top
            gui::draw_rect(fb, dx, dy, 1, dialog_h as usize, 0x00_FF_FF_FF, None); // Left
            gui::draw_rect(fb, dx + dialog_w - 1, dy, 1, dialog_h as usize, 0x00_FF_FF_FF, None); // Right
            gui::draw_rect(fb, dx, dy + dialog_h - 1, dialog_w as usize, 1, 0x00_FF_FF_FF, None); // Bottom

            font::draw_string(fb, dx + 20, dy + 20, "Format Drive?", 0x00_FF_FF_00, None);
            font::draw_string(fb, dx + 20, dy + 45, "All data will be lost!", 0x00_FF_55_55, None);

            // Dialog Buttons
            Button::new(dx + 20, dy + 75, 105, 30, "Yes, Format").draw(fb, 0, 0, None);
            Button::new(dx + 135, dy + 75, 105, 30, "Cancel").draw(fb, 0, 0, None);
        }
    }

    fn handle_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::MouseClick { x, y, width, height } => {
                if self.show_confirm {
                    let dialog_w = 260;
                    let dialog_h = 120;
                    // Centering logic matching the draw method, relative to content area
                    let rel_dx = (*width as isize - dialog_w) / 2;
                    let rel_dy = (*height as isize - dialog_h) / 2 - 22; // Offset for titlebar

                    let btn_yes = Button::new(rel_dx + 20, rel_dy + 75, 105, 30, "Yes, Format");
                    let btn_no = Button::new(rel_dx + 135, rel_dy + 75, 105, 30, "Cancel");

                    if btn_yes.contains(*x, *y) {
                        self.format_drive();
                        self.show_confirm = false;
                        self.refresh();
                    } else if btn_no.contains(*x, *y) {
                        self.show_confirm = false;
                    }
                    return;
                }

                // AppEvent coordinates are relative to the window content area.
                // Window titlebar is ~22px. In draw(), we draw at win.y + win.height - 40.
                // So relative y is height - 40.
                let btn_y = (*height as isize) - 40; 
                
                // Recreate buttons at relative coordinates to check hits
                let btn_init = Button::new(10, btn_y, 140, 30, "Init MBR");
                let btn_fmt = Button::new(160, btn_y, 160, 30, "Format NebulaFS");
                
                if btn_init.contains(*x, *y) {
                    self.initialize_mbr();
                    self.refresh();
                } else if btn_fmt.contains(*x, *y) {
                    self.show_confirm = true;
                }
            }
            _ => {}
        }
    }

    fn box_clone(&self) -> Box<dyn App> {
        Box::new(self.clone())
    }
}

impl PartitionManager {
    fn initialize_mbr(&self) {
        let drive = AtaDrive::new(true, true);
        let mut mbr = [0u8; 512];
        // 0x55AA Signature
        mbr[510] = 0x55;
        mbr[511] = 0xAA;
        
        // Create one partition spanning the "whole" disk (fake size for now)
        // Entry 1 at offset 446
        mbr[446] = 0x80; // Active
        mbr[446 + 4] = 0x83; // Type Linux (generic)
        // LBA Start = 2048 (1MB alignment)
        mbr[446 + 8] = 0x00;
        mbr[446 + 9] = 0x08;
        mbr[446 + 10] = 0x00;
        mbr[446 + 11] = 0x00;
        // Size = 204800 sectors (100MB)
        mbr[446 + 12] = 0x00;
        mbr[446 + 13] = 0x20;
        mbr[446 + 14] = 0x03;
        mbr[446 + 15] = 0x00;

        drive.write_sectors(0, &mbr);
    }

    fn format_drive(&self) {
        let drive = Arc::new(AtaDrive::new(true, true));
        let mut root_vdev = Vdev::new_leaf(0, 0, "ata0", 0, 9);
        root_vdev.backend = Some(drive.clone());
        
        // Format the drive using the new NebulaFS format function
        let _ = nebulafs::fs::NebulaFileSystem::format(root_vdev, "pool0");
    }
}