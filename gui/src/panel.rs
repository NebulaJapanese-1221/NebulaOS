use crate::gui::Control;

pub struct Panel {
    pub control: Control,
}

impl Panel {
    pub const fn new(control: Control) -> Self {
        Panel { control }
    }
}
