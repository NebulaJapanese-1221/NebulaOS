use crate::gui::widget::Widget;

pub struct Checkbox {
    pub widget: Widget,
    pub checked: bool,
}

impl Checkbox {
    pub const fn new(widget: Widget, checked: bool) -> Self {
        Checkbox { widget, checked }
    }
}
