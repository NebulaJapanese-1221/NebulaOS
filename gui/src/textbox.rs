use crate::control::Control;

pub struct TextBox {
    pub control: Control,
    pub text: *mut u8,
}

impl TextBox {
    pub const fn new(control: Control, text: *mut u8) -> Self {
        TextBox { control, text }
    }
}
