pub struct Window {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub title: *const u8,
}

impl Window {
    pub const fn new(x: i32, y: i32, width: u32, height: u32, title: *const u8) -> Self {
        Window { x, y, width, height, title }
    }
}
