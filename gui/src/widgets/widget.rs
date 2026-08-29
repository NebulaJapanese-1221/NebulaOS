use crate::gui::Control;

pub struct Widget {
    pub control: Control,
}

impl Widget {
    pub const fn new(control: Control) -> Self {
        Widget { control }
    }
}
