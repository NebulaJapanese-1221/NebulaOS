use crate::widgets::widget::Widget;

pub struct ProgressBar {
    pub widget: Widget,
    pub value: u32,
    pub max: u32,
}

impl ProgressBar {
    pub const fn new(widget: Widget, value: u32, max: u32) -> Self {
        ProgressBar { widget, value, max }
    }
}
