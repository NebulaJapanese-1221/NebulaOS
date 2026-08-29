use crate::gui::Window;

pub struct FileManager {
    pub window: Window,
}

impl FileManager {
    pub const fn new(window: Window) -> Self {
        FileManager { window }
    }
}
