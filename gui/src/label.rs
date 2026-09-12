use crate::control::Control;

pub struct Label {
    pub control: Control,
    pub text: *const u8,
}

impl Label {
    pub const fn new(control: Control, text: *const u8) -> Self {
        Label { control, text }
    }
}
