pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rectangle {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Rectangle { x, y, width, height }
    }
}
