// Interrupt handler stubs - Architecture-specific implementations in src/arch/

#[cfg(target_arch = "x86")]
pub use crate::arch::x86::interrupts::*;

#[cfg(target_arch = "x86_64")]
pub use crate::arch::x86_64::interrupts::*;
