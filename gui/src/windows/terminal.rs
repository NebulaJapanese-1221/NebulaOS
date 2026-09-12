use crate::window::Window;

pub struct Terminal {
    pub window: Window,
}

impl Terminal {
    pub const fn new(window: Window) -> Self {
        Terminal { window }
    }
}
