use alloc::string::String;
use core::arch::asm;
use spin::Mutex;
use core::sync::atomic::{AtomicUsize, AtomicBool, AtomicU32, Ordering};

pub static CPU_BRAND: Mutex<Option<String>> = Mutex::new(None);
// Global atomic to store current CPU usage percentage (0-100).
pub static CPU_USAGE: AtomicUsize = AtomicUsize::new(0);
pub static IS_IDLE: AtomicBool = AtomicBool::new(false);
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

    // Detect CPU Features using CPUID EAX=1
    let ecx: u32;
    let edx: u32;
    unsafe {
        asm!("cpuid", inout("eax") 1u32 => _, out("ebx") _, out("ecx") ecx, out("edx") edx);
    }

    let mut feat_mask = 0u32;
    if (edx & (1 << 0)) != 0 { feat_mask |= crate::kernel::FEATURE_FPU; }
    if (edx & (1 << 25)) != 0 { feat_mask |= crate::kernel::FEATURE_SSE; }
    if (edx & (1 << 26)) != 0 { feat_mask |= crate::kernel::FEATURE_SSE2; }
    if (ecx & (1 << 0)) != 0 { feat_mask |= crate::kernel::FEATURE_SSE3; }
    if (ecx & (1 << 9)) != 0 { feat_mask |= crate::kernel::FEATURE_SSSE3; }
    if (ecx & (1 << 19)) != 0 { feat_mask |= crate::kernel::FEATURE_SSE4_1; }
    if (ecx & (1 << 20)) != 0 { feat_mask |= crate::kernel::FEATURE_SSE4_2; }
    if (ecx & (1 << 28)) != 0 { feat_mask |= crate::kernel::FEATURE_AVX; }

    // Detect NX (No-Execute) support
    let mut edx_ext: u32;
    unsafe {
        asm!("cpuid", inout("eax") 0x80000001u32 => _, out("ebx") _, out("ecx") _, out("edx") edx_ext);
    }
    if (edx_ext & (1 << 20)) != 0 { feat_mask |= crate::kernel::FEATURE_NX; }

    crate::kernel::CONFIG.features.store(feat_mask, Ordering::SeqCst);

    // Diagnostic log for detected features
    if feat_mask & crate::kernel::FEATURE_AVX != 0 { crate::log_info!("CPU Feature: AVX supported"); }
    if feat_mask & crate::kernel::FEATURE_SSE4_2 != 0 { crate::log_info!("CPU Feature: SSE4.2 supported"); }

    init_fpu();
}

/// Internal helper to measure TSC cycles over a specific number of PIT ticks (ms).
fn measure_tsc_delta(ms: usize) -> u64 {
    // Wait for a fresh tick boundary to ensure we measure a full duration
    let start_ticks = crate::kernel::process::TICKS.load(Ordering::SeqCst);
    while crate::kernel::process::TICKS.load(Ordering::SeqCst) == start_ticks {
        unsafe { asm!("int 0x80", in("eax") 0usize); } // Yield while waiting for PIT
    }

    let start_tsc = read_tsc();
    let target_ticks = crate::kernel::process::TICKS.load(Ordering::SeqCst) + ms;
    
    while crate::kernel::process::TICKS.load(Ordering::SeqCst) < target_ticks {
        unsafe { asm!("int 0x80", in("eax") 0usize); } // Yield
    }
    let end_tsc = read_tsc();
    end_tsc.saturating_sub(start_tsc)
}

/// Calibrates the TSC frequency using the PIT (assumed to be 1000Hz).
pub fn calibrate_tsc() {
    // Take two quick samples to check for stability
    let sample1 = measure_tsc_delta(10) * 100;
    let sample2 = measure_tsc_delta(10) * 100;

    let diff = if sample1 > sample2 { sample1 - sample2 } else { sample2 - sample1 };
    
    // Stability Threshold: 1% variance. 
    // If the clock is jumping around, we perform a much longer measurement.
    let final_freq = if diff > (sample1 / 100) {
        crate::log_warn!("TSC frequency unstable (jitter: {} Hz). Performing high-precision calibration...", diff);
        measure_tsc_delta(100) * 10
    } else {
        (sample1 + sample2) / 2
    };

    crate::kernel::CONFIG.tsc_frequency.store(final_freq as u32, Ordering::SeqCst);
    
    let mhz = final_freq / 1_000_000;
    crate::log_info!("TSC Calibrated: {} MHz", mhz);
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
    let threshold = (crate::kernel::CONFIG.tsc_frequency.load(Ordering::Relaxed) / 10) as u32;
    let total = TOTAL_CYCLES.load(Ordering::Relaxed);
    if total > threshold {
        let idle = IDLE_CYCLES.swap(0, Ordering::Relaxed);
        TOTAL_CYCLES.store(0, Ordering::Relaxed);

        let usage = if total > 0 { 100 - (idle * 100 / total) as usize } else { 0 };
        CPU_USAGE.store(usage, Ordering::Relaxed);
    }
}

/// Enables the No-Execute (NX) bit in the EFER MSR if supported.
pub fn enable_nx() {
    if crate::kernel::CONFIG.has_feature(crate::kernel::FEATURE_NX) {
        unsafe {
            let mut low: u32;
            let mut high: u32;
            asm!("rdmsr", in("ecx") 0xC0000080u32, out("eax") low, out("edx") high);
            low |= 1 << 11; // NXE bit
            asm!("wrmsr", in("ecx") 0xC0000080u32, in("eax") low, in("edx") high);
        }
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

        // Enable SSE: OSFXSR (bit 9) and OSXMMEXCPT (bit 10)
        cr4 |= (1 << 9) | (1 << 10);

        // Enable AVX support if detected in CONFIG
        if crate::kernel::CONFIG.has_feature(crate::kernel::FEATURE_AVX) {
            cr4 |= 1 << 18; // Set OSXSAVE (bit 18) to allow XGETBV/XSETBV
        }

        asm!("mov cr4, {}", in(reg) cr4, options(nomem, nostack));

        if crate::kernel::CONFIG.has_feature(crate::kernel::FEATURE_AVX) {
            let mut eax: u32;
            let mut edx: u32;
            // Read XCR0 (Extended Control Register 0)
            asm!("xgetbv", in("ecx") 0, out("eax") eax, out("edx") edx, options(nomem, nostack));
            eax |= 0x07; // Set bit 0 (x87), bit 1 (SSE), and bit 2 (AVX)
            asm!("xsetbv", in("ecx") 0, in("eax") eax, in("edx") edx, options(nomem, nostack));
        }
    }
}