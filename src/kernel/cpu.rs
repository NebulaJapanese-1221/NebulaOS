use alloc::string::String;
use core::arch::asm;
use spin::Mutex;
use core::sync::atomic::{AtomicUsize, AtomicBool, AtomicU32, Ordering};

pub static CPU_BRAND: Mutex<Option<String>> = Mutex::new(None);
// Global atomic to store current CPU usage percentage (0-100).
pub static CPU_USAGE: AtomicUsize = AtomicUsize::new(0);
// Global atomic to store current CPU temperature in Celsius.
pub static CPU_TEMP: AtomicUsize = AtomicUsize::new(0);
pub static IS_IDLE: AtomicBool = AtomicBool::new(false);
pub static POWER_SAVE: AtomicBool = AtomicBool::new(false);

static IDLE_CYCLES: AtomicU32 = AtomicU32::new(0);
static TOTAL_CYCLES: AtomicU32 = AtomicU32::new(0);
pub static LOOPS_PER_MS: AtomicUsize = AtomicUsize::new(2000000); // Sensible fallback for ~2GHz CPUs
pub static CYCLES_PER_US: AtomicUsize = AtomicUsize::new(2000);    // Default 2MHz for TSC fallback

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
}

pub fn read_tsc() -> u64 {
    let mut low: u32;
    let mut high: u32;
    unsafe {
        asm!("rdtsc", out("eax") low, out("edx") high, options(nomem, nostack));
    }
    ((high as u64) << 32) | (low as u64)
}

/// Returns how many milliseconds a single PIT tick represents.
/// (1ms at 1000Hz, 10ms at 100Hz)
pub fn get_tick_increment() -> usize {
    if POWER_SAVE.load(Ordering::Relaxed) { 10 } else { 1 }
}

/// Calibrates the NOP loop based on the system PIT timer.
pub fn calibrate_delay() {
    let start_tick = crate::kernel::process::TICKS.load(Ordering::Relaxed);
    // Wait for the next tick to start to ensure we have a full window
    while crate::kernel::process::TICKS.load(Ordering::Relaxed) == start_tick {
        unsafe { core::arch::asm!("pause"); }
    }
    
    let start_tsc = read_tsc();
    let start_time = crate::kernel::process::TICKS.load(Ordering::Relaxed);
    let mut count = 0usize;
    while crate::kernel::process::TICKS.load(Ordering::Relaxed) < start_time + 100 { // Measure over 100ms
        unsafe { core::arch::asm!("nop"); }
        count += 1;
    }
    let end_tsc = read_tsc();

    LOOPS_PER_MS.store(count / 100, Ordering::Relaxed);
    // Cycles per microsecond = (Total Cycles / 100ms) / 1000
    CYCLES_PER_US.store(((end_tsc - start_tsc) / 100000) as usize, Ordering::Relaxed);
    crate::serial_println!("[CPU] Calibration complete: {} loops/ms, {} TSC MHz", count / 100, (end_tsc - start_tsc) / 100000);
}

/// Performs a calibrated busy-wait delay. Useful when interrupts are disabled.
pub fn spin_wait_ms(ms: usize) {
    let factor = LOOPS_PER_MS.load(Ordering::Relaxed);
    for _ in 0..(ms * factor) {
        unsafe { core::arch::asm!("nop"); }
    }
}

/// Performs a high-precision delay using the TSC. Does not rely on interrupts.
pub fn spin_wait_us(us: usize) {
    let start = read_tsc();
    let cycles_to_wait = us as u64 * CYCLES_PER_US.load(Ordering::Relaxed) as u64;
    while read_tsc() - start < cycles_to_wait {
        unsafe { core::arch::asm!("pause"); }
    }
}

pub fn accumulate_usage(cycles: u64, is_idle: bool) {
    // Safely cast to u32. In 1ms (tick), cycles ~ millions, fits in u32.
    let cycles_u32 = cycles as u32;
    TOTAL_CYCLES.fetch_add(cycles_u32, Ordering::Relaxed);
    if is_idle {
        IDLE_CYCLES.fetch_add(cycles_u32, Ordering::Relaxed);
    }
}

pub fn update_usage_stats() {
    let total = TOTAL_CYCLES.swap(0, Ordering::Relaxed);
    if total > 0 {
        let idle = IDLE_CYCLES.swap(0, Ordering::Relaxed);

        let usage = if total > 0 { 100 - (idle * 100 / total) as usize } else { 0 };
        CPU_USAGE.store(usage, Ordering::Relaxed);
    }
}