pub mod font;
pub mod font_jp;

// Import items from gui to make them available to child modules (font.rs) via `super`
use crate::userspace::gui::rect;
use crate::userspace::gui::LARGE_TEXT;