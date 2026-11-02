// Kernel drivers module

#[path = "../allocator.rs"]
pub mod allocator;
pub mod boot_screen;
pub mod disk;
pub mod font;
pub mod framebuffer;
pub mod keyboard;
pub mod mouse;
#[path = "../paging.rs"]
pub mod paging;
#[path = "../pic.rs"]
pub mod pic;
#[path = "../pit.rs"]
pub mod pit;
#[path = "../process.rs"]
pub mod process;
pub mod serial;
#[path = "../syscall.rs"]
pub mod syscall;
pub mod user_process;
pub mod wm;
