// PS/2 controller I/O - delegates to architecture-specific port I/O

#[cfg(target_arch = "x86")]
pub use crate::arch::x86::io::*;

#[cfg(target_arch = "x86_64")]
pub use crate::arch::x86_64::io::*;
