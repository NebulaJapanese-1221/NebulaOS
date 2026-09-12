use crate::widgets::widget::Widget;

pub struct RadioButton {
    pub widget: Widget,
    pub selected: bool,
}

impl RadioButton {
    pub const fn new(widget: Widget, selected: bool) -> Self {
        RadioButton { widget, selected }
    }
}
