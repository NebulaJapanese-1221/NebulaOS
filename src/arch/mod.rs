// Architecture-specific modules for NebulaOS
// Supports x86 (32-bit) and x86_64 (64-bit) architectures

#[cfg(target_arch = "x86")]
pub mod x86;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

