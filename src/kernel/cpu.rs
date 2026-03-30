use alloc::string::String;
use core::arch::asm;
use spin::Mutex;
use core::sync::atomic::{AtomicUsize, AtomicBool, AtomicU32, AtomicU64, Ordering};

pub static CPU_BRAND: Mutex<Option<String>> = Mutex::new(None);
// Global atomic to store current CPU usage percentage (0-100).
pub static CPU_USAGE: AtomicUsize = AtomicUsize::new(0);
pub static IS_IDLE: AtomicBool = AtomicBool::new(false);
/// Measured TSC cycles per second. Defaults to 2GHz until calibrated.
pub static TSC_FREQUENCY: AtomicU64 = AtomicU64::new(2_000_000_000);

static IDLE_CYCLES: AtomicU32 = AtomicU32::new(0);
static TOTAL_CYCLES: AtomicU32 = AtomicU32::new(0);

pub fn init() {
    let mut brand = String::with_capacity(48);
    
    // Check maximum extended function
    let mut eax: u32;
    unsafe { 
        asm!("cpuid", inout("eax") 0x80000000u32 => eax, out("ebx") _, out("ecx") _, out("edx") _); 
    }
    
    if eax >= 0x80000004 {
        // Execute CPUID 0x80000002..0x80000004 to get the brand string
        for i in 0x80000002u32..=0x80000004u32 {
            let mut a: u32;
            let mut b: u32;
            let mut c: u32;
            let mut d: u32;
            unsafe { 
                asm!("cpuid", inout("eax") i => a, out("ebx") b, out("ecx") c, out("edx") d);
            }
            
            // Append bytes to string
            for val in [a, b, c, d] {
                for byte in val.to_le_bytes() {
                    if byte != 0 {
                        brand.push(byte as char);
                    }
                }
            }
        }
    } else {
        brand.push_str("Generic x86 Processor");
    }
    
    *CPU_BRAND.lock() = Some(brand.trim().into());

    init_fpu();
}

/// Calibrates the TSC frequency using the PIT (assumed to be 1000Hz).
pub fn calibrate_tsc() {
    // Wait for a baseline tick
    let start_ticks = crate::kernel::process::TICKS.load(Ordering::SeqCst);
    while crate::kernel::process::TICKS.load(Ordering::SeqCst) == start_ticks {
        core::hint::spin_loop();
    }

    let start_tsc = read_tsc();
    // Measure over 10ms (10 ticks)
    let target_ticks = crate::kernel::process::TICKS.load(Ordering::SeqCst) + 10;
    while crate::kernel::process::TICKS.load(Ordering::SeqCst) < target_ticks {
        core::hint::spin_loop();
    }
    let end_tsc = read_tsc();

    let freq = (end_tsc - start_tsc) * 100; // (cycles / 10ms) * 100 = cycles / 1s
    TSC_FREQUENCY.store(freq, Ordering::SeqCst);
}

pub fn read_tsc() -> u64 {
    let mut low: u32;
    let mut high: u32;
    unsafe {
        asm!("rdtsc", out("eax") low, out("edx") high, options(nomem, nostack));
    }
    ((high as u64) << 32) | (low as u64)
}

pub fn accumulate_usage(cycles: u64, is_idle: bool) {
    // Safely cast to u32. In 1ms (tick), cycles ~ millions, fits in u32.
    let cycles_u32 = cycles as u32;
    TOTAL_CYCLES.fetch_add(cycles_u32, Ordering::Relaxed);
    if is_idle {
        IDLE_CYCLES.fetch_add(cycles_u32, Ordering::Relaxed);
    }

    // Calculate usage periodically (every ~100M cycles is fine, or based on total accumulation)
    // Calculate every 100ms based on measured frequency.
    let threshold = (TSC_FREQUENCY.load(Ordering::Relaxed) / 10) as u32;
    let total = TOTAL_CYCLES.load(Ordering::Relaxed);
    if total > threshold {
        let idle = IDLE_CYCLES.swap(0, Ordering::Relaxed);
        TOTAL_CYCLES.store(0, Ordering::Relaxed);

        let usage = if total > 0 { 100 - (idle * 100 / total) as usize } else { 0 };
        CPU_USAGE.store(usage, Ordering::Relaxed);
    }
}

/// Initializes the FPU (Floating Point Unit) and enables SSE.
pub fn init_fpu() {
    unsafe {
        let mut cr0: u32;
        asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack));
        // Clear EM (bit 2) to indicate FPU is present
        // Set MP (bit 1) to control interaction of WAIT/FWAIT
        cr0 = (cr0 & !(1 << 2)) | (1 << 1);
        asm!("mov cr0, {}", in(reg) cr0, options(nomem, nostack));

        let mut cr4: u32;
        asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack));
        // Set OSFXSR (bit 9) and OSXMMEXCPT (bit 10) to enable SSE
        cr4 |= (1 << 9) | (1 << 10);
        asm!("mov cr4, {}", in(reg) cr4, options(nomem, nostack));
    }
}