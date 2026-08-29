use core::ffi::c_void;

pub const NEBULAOS_VERSION: &str = "0.0.1";
pub const NEBULAOS_NAME: &str = "NebulaOS";

#[cfg(target_arch = "x86")]
pub const KERNEL_MODE: u32 = 32;

#[cfg(target_arch = "x86_64")]
pub const KERNEL_MODE: u32 = 64;

pub const NEBULAOS_SUCCESS: i32 = 0;
pub const NEBULAOS_ERROR: i32 = -1;
pub const NEBULAOS_ENOMEM: i32 = -2;
pub const NEBULAOS_EINVAL: i32 = -3;
pub const NEBULAOS_ENOSYS: i32 = -4;

pub const KERN_EMERG: u32 = 0;
pub const KERN_ALERT: u32 = 1;
pub const KERN_CRIT: u32 = 2;
pub const KERN_ERR: u32 = 3;
pub const KERN_WARNING: u32 = 4;
pub const KERN_NOTICE: u32 = 5;
pub const KERN_INFO: u32 = 6;
pub const KERN_DEBUG: u32 = 7;

#[macro_export]
macro_rules! PACKED {
    ($(#[$attr:meta])* $vis:vis struct $name:ident { $($fields:tt)* }) => {
        $(#[$attr])*
        $vis struct $name { $($fields)* }
    };
}

#[macro_export]
macro_rules! ALIGNED {
    ($n:expr, $t:ty) => {
        struct Aligned<$t> {
            _align: [u8; $n],
            _data: $t,
        }
    };
}

pub unsafe fn kernel_panic(message: &str) -> ! {
    use crate::common::vga;
    vga::set_color(vga::VGA_COLOR_RED);
    vga::set_bg_color(vga::VGA_COLOR_BLACK);
    vga::puts("\n\nKERNEL PANIC: ");
    vga::puts(message);
    vga::puts("\n\nSystem halted.");
    loop {
        unsafe { asm!("hlt"); }
    }
}
