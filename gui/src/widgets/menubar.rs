use crate::widgets::widget::Widget;

pub struct MenuBar {
    pub widget: Widget,
}

impl MenuBar {
    pub const fn new(widget: Widget) -> Self {
        MenuBar { widget }
    }
}
