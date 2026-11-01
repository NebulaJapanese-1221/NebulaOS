// Kernel drivers module

pub mod keyboard;
pub mod mouse;
pub mod disk;
pub mod font;
pub mod framebuffer;
pub mod serial;
pub mod wm;
#[path = "../pic.rs"]
pub mod pic;
#[path = "../pit.rs"]
pub mod pit;
#[path = "../paging.rs"]
pub mod paging;
#[path = "../allocator.rs"]
pub mod allocator;
#[path = "../syscall.rs"]
pub mod syscall;
#[path = "../process.rs"]
pub mod process;
