// Kernel drivers module

pub mod vga;
pub mod keyboard;
pub mod mouse;
pub mod vesa;
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

pub use vga::print;
pub use vga::write_char as print_char;
