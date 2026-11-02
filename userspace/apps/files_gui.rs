use super::gui::{Application, Button, Canvas, Event, PointerEvent, Rect, ScrollArea, TextField};
use crate::storage::{self, FileInfo, StorageError};

const FILE_SLOTS: usize = 16;

pub struct FileManager {
    name: [u8; 16],
    name_len: usize,
    content: [u8; 128],
    content_len: usize,
    selected: Option<usize>,
    files: [FileInfo; FILE_SLOTS],
    name_field: TextField,
    content_field: TextField,
    list_scroll: ScrollArea,
    new_button: Button,
    save_button: Button,
    delete_button: Button,
    rename_button: Button,
    format_button: Button,
    confirm_format_button: Button,
    format_armed: bool,
    row_down: bool,
    row_armed: Option<usize>,
    status: &'static str,
}

impl FileManager {
    pub fn new() -> Self {
        let mut manager = Self {
            name: [0; 16],
            name_len: 0,
            content: [0; 128],
            content_len: 0,
            selected: None,
            files: [FileInfo {
                name: [0; 16],
                length: 0,
            }; FILE_SLOTS],
            name_field: TextField::new(Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            }),
            content_field: TextField::new(Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            }),
            list_scroll: ScrollArea::new(),
            new_button: Button::new("NEW"),
            save_button: Button::new("SAVE"),
            delete_button: Button::new("DELETE"),
            rename_button: Button::new("RENAME"),
            format_button: Button::new("FORMAT"),
            confirm_format_button: Button::new("CONFIRM FORMAT"),
            format_armed: false,
            row_down: false,
            row_armed: None,
            status: "IDE DISK STATUS UNKNOWN",
        };
        manager.refresh();
        manager
    }

    fn refresh(&mut self) {
        match storage::list_files() {
            Ok(files) => {
                self.files = files;
                self.status = "NEBULAFS READY";
            }
            Err(StorageError::NoDisk) => self.status = "NO IDE DISK ATTACHED",
            Err(StorageError::NotFormatted) => self.status = "DISK NEEDS FORMATTING",
            Err(StorageError::Io) => self.status = "DISK I/O ERROR",
            Err(_) => self.status = "STORAGE UNAVAILABLE",
        }
    }

    fn selected_name(&self) -> Option<&[u8]> {
        let index = self.selected?;
        let file = self.files.get(index)?;
        let length = file.name.iter().position(|byte| *byte == 0).unwrap_or(16);
        Some(&file.name[..length])
    }

    fn execute(&mut self, result: Result<(), StorageError>, success: &'static str) {
        match result {
            Ok(()) => {
                self.status = success;
                self.refresh();
                self.status = success;
            }
            Err(StorageError::NoDisk) => self.status = "NO IDE DISK ATTACHED",
            Err(StorageError::NotFormatted) => self.status = "FORMAT A BLANK DISK FIRST",
            Err(StorageError::NotBlank) => self.status = "REFUSING TO OVERWRITE DISK",
            Err(StorageError::InvalidName) => self.status = "USE A-Z, 0-9, DOT, DASH OR _",
            Err(StorageError::FileTooLarge) => self.status = "FILE LIMIT IS 512 BYTES",
            Err(StorageError::DirectoryFull) => self.status = "DISK DIRECTORY IS FULL",
            Err(StorageError::NotFound) => self.status = "FILE NOT FOUND",
            Err(StorageError::Io) => self.status = "DISK I/O ERROR",
        }
    }

    fn set_name(&mut self, name: &[u8]) {
        self.name = [0; 16];
        self.name_len = name.len().min(15);
        self.name[..self.name_len].copy_from_slice(&name[..self.name_len]);
    }

    fn row_rect(area: Rect, index: usize, offset: u32) -> Rect {
        Rect {
            x: area.x + 18,
            y: (area.y + 82 + index as u32 * 23).saturating_sub(offset),
            width: 190,
            height: 21,
        }
    }
}

impl Application for FileManager {
    fn title(&self) -> &'static str {
        "FILE MANAGER"
    }

    fn render(&self, canvas: &mut dyn Canvas, area: Rect, pointer: PointerEvent) {
        canvas.draw_text(area.x + 18, area.y + 10, "NAME", 0x1596B5, 1);
        let name_rect = Rect {
            x: area.x + 18,
            y: area.y + 26,
            width: 170,
            height: 26,
        };
        let mut name_field = TextField::new(name_rect);
        name_field.focused = self.name_field.focused;
        name_field.render(
            canvas,
            core::str::from_utf8(&self.name[..self.name_len]).unwrap_or(""),
        );
        let content_rect = Rect {
            x: area.x + 226,
            y: area.y + 170,
            width: 164,
            height: 54,
        };
        let mut content_field = TextField::new(content_rect);
        content_field.focused = self.content_field.focused;
        content_field.render(
            canvas,
            core::str::from_utf8(&self.content[..self.content_len]).unwrap_or(""),
        );

        let list = Rect {
            x: area.x + 18,
            y: area.y + 58,
            width: 190,
            height: area.height.saturating_sub(72),
        };
        self.list_scroll
            .render(canvas, list, FILE_SLOTS as u32 * 23);
        for (index, file) in self.files.iter().enumerate() {
            if file.name[0] == 0 {
                continue;
            }
            let rect = Self::row_rect(area, index, self.list_scroll.offset);
            let list_bottom = area.y + area.height.saturating_sub(14);
            if rect.y + rect.height <= area.y + 82 || rect.y >= list_bottom {
                continue;
            }
            canvas.fill_rect(
                rect,
                if self.selected == Some(index) {
                    0xB8DCE7
                } else {
                    0xE2E8F0
                },
            );
            let name_len = file.name.iter().position(|byte| *byte == 0).unwrap_or(16);
            if let Ok(name) = core::str::from_utf8(&file.name[..name_len]) {
                canvas.draw_text(rect.x + 6, rect.y + 7, name, 0x111827, 1);
            }
        }

        let right = area.x + 226;
        draw_button(
            canvas,
            &self.new_button,
            Rect {
                x: right,
                y: area.y + 26,
                width: 78,
                height: 26,
            },
            pointer,
        );
        draw_button(
            canvas,
            &self.save_button,
            Rect {
                x: right + 86,
                y: area.y + 26,
                width: 78,
                height: 26,
            },
            pointer,
        );
        draw_button(
            canvas,
            &self.delete_button,
            Rect {
                x: right,
                y: area.y + 62,
                width: 78,
                height: 26,
            },
            pointer,
        );
        draw_button(
            canvas,
            &self.rename_button,
            Rect {
                x: right + 86,
                y: area.y + 62,
                width: 78,
                height: 26,
            },
            pointer,
        );
        draw_button(
            canvas,
            &self.format_button,
            Rect {
                x: right,
                y: area.y + 98,
                width: 100,
                height: 26,
            },
            pointer,
        );
        if self.format_armed {
            draw_button(
                canvas,
                &self.confirm_format_button,
                Rect {
                    x: right,
                    y: area.y + 134,
                    width: 150,
                    height: 26,
                },
                pointer,
            );
        }
        canvas.draw_text(right, area.y + 232, self.status, 0xA23B45, 1);
        canvas.draw_text(right, area.y + 248, "16 FILES, 512B EACH", 0x111827, 1);
    }

    fn handle_event(&mut self, event: Event, area: Rect) -> bool {
        match event {
            Event::Key(b'\t') => {
                if self.name_field.focused {
                    self.name_field.focused = false;
                    self.content_field.focused = true;
                } else {
                    self.name_field.focused = true;
                    self.content_field.focused = false;
                }
                true
            }
            Event::Key(8) if self.name_field.focused && self.name_len > 0 => {
                self.name_len -= 1;
                true
            }
            Event::Key(8) if self.content_field.focused && self.content_len > 0 => {
                self.content_len -= 1;
                true
            }
            Event::Key(b'\n')
                if self.content_field.focused && self.content_len < self.content.len() =>
            {
                self.content[self.content_len] = b'\n';
                self.content_len += 1;
                true
            }
            Event::Key(key @ (32..=126)) if self.name_field.focused && self.name_len < 15 => {
                self.name[self.name_len] = key.to_ascii_uppercase();
                self.name_len += 1;
                true
            }
            Event::Key(key @ (32..=126))
                if self.content_field.focused && self.content_len < self.content.len() =>
            {
                self.content[self.content_len] = key;
                self.content_len += 1;
                true
            }
            Event::Key(key @ (b'[' | b']')) => self.list_scroll.handle_key(
                key,
                FILE_SLOTS as u32 * 23,
                area.height.saturating_sub(72),
            ),
            Event::Key(_) => false,
            Event::Pointer(pointer) => {
                self.name_field.rect = Rect {
                    x: area.x + 18,
                    y: area.y + 26,
                    width: 170,
                    height: 26,
                };
                let mut changed = self.name_field.handle_pointer(pointer);
                self.content_field.rect = Rect {
                    x: area.x + 226,
                    y: area.y + 170,
                    width: 164,
                    height: 54,
                };
                changed |= self.content_field.handle_pointer(pointer);
                let right = area.x + 226;
                let new = self.new_button.handle_pointer(
                    Rect {
                        x: right,
                        y: area.y + 26,
                        width: 78,
                        height: 26,
                    },
                    pointer,
                );
                let save = self.save_button.handle_pointer(
                    Rect {
                        x: right + 86,
                        y: area.y + 26,
                        width: 78,
                        height: 26,
                    },
                    pointer,
                );
                let delete = self.delete_button.handle_pointer(
                    Rect {
                        x: right,
                        y: area.y + 62,
                        width: 78,
                        height: 26,
                    },
                    pointer,
                );
                let rename = self.rename_button.handle_pointer(
                    Rect {
                        x: right + 86,
                        y: area.y + 62,
                        width: 78,
                        height: 26,
                    },
                    pointer,
                );
                let format = self.format_button.handle_pointer(
                    Rect {
                        x: right,
                        y: area.y + 98,
                        width: 100,
                        height: 26,
                    },
                    pointer,
                );
                let confirm = self.confirm_format_button.handle_pointer(
                    Rect {
                        x: right,
                        y: area.y + 134,
                        width: 150,
                        height: 26,
                    },
                    pointer,
                );
                if new {
                    if self.name_len == 0 {
                        self.status = "ENTER A FILE NAME";
                    } else {
                        self.content_len = 0;
                        self.content_field.focused = true;
                        self.name_field.focused = false;
                        self.selected = None;
                        self.status = "NEW FILE READY; ENTER CONTENT";
                    }
                    changed = true;
                }
                if save {
                    self.execute(
                        storage::write_file(
                            &self.name[..self.name_len],
                            &self.content[..self.content_len],
                        ),
                        "FILE SAVED",
                    );
                    changed = true;
                }
                if delete {
                    if let Some(name) = self.selected_name() {
                        let mut name_copy = [0; 16];
                        name_copy[..name.len()].copy_from_slice(name);
                        self.execute(
                            storage::delete_file(&name_copy[..name.len()]),
                            "FILE DELETED",
                        );
                        self.selected = None;
                    } else {
                        self.status = "SELECT A FILE";
                    }
                    changed = true;
                }
                if rename {
                    if let Some(old_name) = self.selected_name() {
                        let mut old_name_copy = [0; 16];
                        old_name_copy[..old_name.len()].copy_from_slice(old_name);
                        self.execute(
                            storage::rename_file(
                                &old_name_copy[..old_name.len()],
                                &self.name[..self.name_len],
                            ),
                            "FILE RENAMED",
                        );
                    } else {
                        self.status = "SELECT A FILE";
                    }
                    changed = true;
                }
                if format {
                    self.format_armed = true;
                    self.status = "CONFIRM ONLY ON A BLANK DISK";
                    changed = true;
                }
                if confirm && self.format_armed {
                    self.execute(
                        storage::format_if_blank().map(|_| ()),
                        "BLANK DISK FORMATTED",
                    );
                    self.format_armed = false;
                    changed = true;
                }

                let row = (0..FILE_SLOTS).find(|index| {
                    let rect = Self::row_rect(area, *index, self.list_scroll.offset);
                    rect.contains(pointer.x, pointer.y)
                });
                if pointer.left_down && !self.row_down {
                    self.row_armed = row;
                } else if !pointer.left_down && self.row_down && row == self.row_armed {
                    if let Some(index) = row {
                        if self.files[index].name[0] != 0 {
                            let name_len = self.files[index]
                                .name
                                .iter()
                                .position(|byte| *byte == 0)
                                .unwrap_or(16);
                            let mut selected_name = [0; 16];
                            selected_name[..name_len]
                                .copy_from_slice(&self.files[index].name[..name_len]);
                            self.set_name(&selected_name[..name_len]);
                            self.selected = Some(index);
                            let mut content = [0; 512];
                            match storage::read_file(&self.name[..self.name_len], &mut content) {
                                Ok(length) => {
                                    self.content_len = length.min(self.content.len());
                                    self.content[..self.content_len]
                                        .copy_from_slice(&content[..self.content_len]);
                                    self.status = "FILE SELECTED";
                                }
                                Err(_) => self.status = "FILE READ ERROR",
                            }
                            changed = true;
                        }
                    }
                    self.row_armed = None;
                }
                self.row_down = pointer.left_down;
                changed
            }
        }
    }
}

fn draw_button(canvas: &mut dyn Canvas, button: &Button, rect: Rect, pointer: PointerEvent) {
    button.render(canvas, rect, pointer);
}
