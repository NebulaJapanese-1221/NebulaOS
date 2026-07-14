use crate::framebuffer::{Framebuffer, Rect};
use crate::gui::{draw_string, TITLE_BAR_HEIGHT};
use alloc::string::String;
use alloc::vec::Vec;
use crate::fs::{NebulaFS, FileSystemOps};

#[derive(Debug)]
pub struct FileManagerState {
    pub current_path: String,
    pub files: Vec<String>,
    pub selected_file: Option<usize>,
    pub scroll_offset: usize,
    pub fs: Option<NebulaFS>, // Add filesystem reference
}

impl FileManagerState {
    pub fn new() -> Self {
        Self {
            current_path: String::from("/"),
            files: Vec::new(),
            selected_file: None,
            scroll_offset: 0,
            fs: None, // Will be set when the file manager is initialized
        }
    }

    pub fn set_filesystem(&mut self, fs: NebulaFS) {
        self.fs = Some(fs);
    }

    pub fn refresh_files(&mut self) {
        self.files.clear();
        self.files.push(String::from("."));
        self.files.push(String::from(".."));

        // If we have a filesystem, try to read the directory
        if let Some(fs) = &self.fs {
            // In a real implementation, we would read the directory contents
            // For now, we'll simulate some files

            // Try to create some test files if they don't exist
            let test_files = ["test1.txt", "test2.txt", "test3.txt"];
            for file in &test_files {
                let _ = fs.create_file(2, file); // 2 is root directory inode
            }

            // Add directories
            let test_dirs = ["Documents", "Downloads", "Pictures"];
            for dir in &test_dirs {
                let _ = fs.create_dir(2, dir);
                self.files.push(String::from(*dir));
            }

            // Add files
            for file in &test_files {
                self.files.push(String::from(*file));
            }
        }
    }
    pub fn handle_keypress(&mut self, c: char) {
        match c {
            'j' => { // Move down
                if let Some(selected) = self.selected_file {
                    if selected + 1 < self.files.len() {
                        self.selected_file = Some(selected + 1);
                        
                        // Scroll if needed
                        if selected + 1 >= self.scroll_offset + 10 {
                            self.scroll_offset += 1;
                        }
                    }
                } else {
                    self.selected_file = Some(0);
                }
            }
            'k' => { // Move up
                if let Some(selected) = self.selected_file {
                    if selected > 0 {
                        self.selected_file = Some(selected - 1);
                        
                        // Scroll if needed
                        if selected <= self.scroll_offset {
                            self.scroll_offset = self.scroll_offset.saturating_sub(1);
                        }
                    }
                }
            }
            '\n' => { // Enter - open file/directory
                if let Some(selected) = self.selected_file {
                    let filename = &self.files[selected];
                    if filename == ".." {
                        // Go up one directory
                        if self.current_path != "/" {
                            let mut parts: Vec<&str> = self.current_path.trim_matches('/').split('/').collect();
                            parts.pop();
                            self.current_path = if parts.is_empty() {
                                String::from("/")
                            } else {
                                format!("/{}", parts.join("/"))
                            };
                            self.refresh_files();
                        }
                    } else if filename != "." {
                        // Try to open the file/directory
                        if let Some(fs) = &mut self.fs {
                            // Check if it's a directory by trying to lookup
                            match fs.lookup(2, filename) { // 2 is typically the root directory inode
                                Ok(_) => {
                                    // It exists, assume it's a directory for now
                            self.current_path = if self.current_path == "/" {
                                format!("/{}", filename)
                            } else {
                                format!("{}/{}", self.current_path, filename)
                            };
                            self.refresh_files();
                        }
                                Err(_) => {
                                    // It's a file - show a message
                                    println!("Opening file: {}", filename);
                    }
                }
            }
        }
    }
}
            'n' => { // New file
                if let Some(fs) = &mut self.fs {
                    let new_filename = "new_file.txt";
                    match fs.create_file(2, new_filename) {
                        Ok(inode) => {
                            println!("Created new file: {} with inode {}", new_filename, inode);
                            self.refresh_files();
                        }
                        Err(e) => {
                            println!("Failed to create file: {}", e);
                        }
                    }
                }
            }
            'd' => { // New directory
                if let Some(fs) = &mut self.fs {
                    let new_dirname = "new_directory";
                    match fs.create_dir(2, new_dirname) {
                        Ok(inode) => {
                            println!("Created new directory: {} with inode {}", new_dirname, inode);
                            self.refresh_files();
                        }
                        Err(e) => {
                            println!("Failed to create directory: {}", e);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct FileManagerApp;

impl FileManagerApp {
    pub fn draw(fb: &mut Framebuffer, bounds: Rect, state: &FileManagerState) {
        let x = bounds.x as usize;
        let y = bounds.y as usize + TITLE_BAR_HEIGHT as usize;
        let w = bounds.width as usize;
        let h = (bounds.height as usize).saturating_sub(TITLE_BAR_HEIGHT as usize);

        // Draw background
        fb.draw_rect(x, y, w, h, 0x00F0F0F0); // Light gray background

        // Draw path bar
        fb.draw_rect(x, y, w, 25, 0x00D0D0D0); // Slightly darker gray
        draw_string(fb, x + 5, y + 5, &format!("Path: {}", state.current_path), 0x00000000);

        // Draw file list
        let mut current_y = y + 30;
        let visible_files = state.files.iter()
            .skip(state.scroll_offset)
            .take(10)
            .enumerate();

        for (i, file) in visible_files {
            let color = if Some(state.scroll_offset + i) == state.selected_file {
                0x000000FF // Blue for selected
            } else {
                0x00000000 // Black for normal
            };
            
            draw_string(fb, x + 5, current_y, file, color);
            current_y += 15;
        }

        // Draw scrollbar if needed
        if state.files.len() > 10 {
            let scrollbar_x = x + w - 15;
            let scrollbar_y = y + 30;
            let scrollbar_height = h - 35;
            
            // Draw scrollbar background
            fb.draw_rect(scrollbar_x, scrollbar_y, 10, scrollbar_height, 0x00C0C0C0);
            
            // Calculate thumb position and size
            let thumb_height = (scrollbar_height as f32 * (10.0 / state.files.len() as f32)) as usize;
            let thumb_y = scrollbar_y + ((state.scroll_offset as f32 / (state.files.len() - 10) as f32) * (scrollbar_height - thumb_height) as f32) as usize;
            
            // Draw scrollbar thumb
            fb.draw_rect(scrollbar_x, thumb_y, 10, thumb_height, 0x00808080);
        }
    }

    pub fn handle_click(state: &mut FileManagerState, bounds: Rect, mx: i32, my: i32) {
        let x = bounds.x as i32;
        let y = bounds.y as i32 + TITLE_BAR_HEIGHT as i32;
        let w = bounds.width as i32;
        let h = (bounds.height as i32) - TITLE_BAR_HEIGHT as i32;

        // Check if click is in the file list area
        if mx >= x && mx <= x + w && my >= y + 30 && my <= y + h {
            let relative_y = my - (y + 30);
            let clicked_index = (relative_y / 15) as usize + state.scroll_offset;
            
            if clicked_index < state.files.len() {
                state.selected_file = Some(clicked_index);
            }
        }
    }

    pub fn handle_keyboard_input(state: &mut FileManagerState, c: char) {
        state.handle_keypress(c);
    }
}