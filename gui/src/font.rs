pub struct Font {
    pub name: *const u8,
    pub size: u32,
}

impl Font {
    pub const fn new(name: *const u8, size: u32) -> Self {
        Font { name, size }
    }
}
