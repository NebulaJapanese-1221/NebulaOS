// CPU exceptions - Architecture-specific implementations in src/arch/

#[cfg(target_arch = "x86")]
pub use crate::arch::x86::exceptions::*;

#[cfg(target_arch = "x86_64")]
pub use crate::arch::x86_64::exceptions::*;

