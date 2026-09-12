use crate::control::Control;

pub struct Button {
    pub control: Control,
    pub text: *const u8,
}

impl Button {
    pub const fn new(control: Control, text: *const u8) -> Self {
        Button { control, text }
    }
}
