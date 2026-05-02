//! AC97 Audio Driver for NebulaOS.

use crate::kernel::{io, pci};
use core::sync::atomic::{AtomicU16, AtomicBool, Ordering};

/// Standard I/O ports for AC97 in QEMU/Bochs (usually assigned via PCI BARs)
static NAM_BASE: AtomicU16 = AtomicU16::new(0x2000);  // Native Audio Mixer
static NABM_BASE: AtomicU16 = AtomicU16::new(0x3000); // Native Audio Bus Master

pub static MASTER_VOLUME: AtomicU16 = AtomicU16::new(80); // Default 80%
pub static IS_MUTED: AtomicBool = AtomicBool::new(false);

/// Updates the master output volume via AC97 NAM register.
pub fn set_master_volume(level: u16) {
    let level = level.min(100);
    MASTER_VOLUME.store(level, Ordering::Relaxed);
    IS_MUTED.store(false, Ordering::Relaxed); // Automatically unmute on manual change
    
    let nam = NAM_BASE.load(Ordering::Relaxed);
    if nam != 0 {
        // AC97 volume is attenuation (0 = max, 0x1F = -46.5dB, 0x3F = mute)
        // We map 100% -> 0 and 0% -> 31 (0x1F) for a usable range.
        let attn = ((100 - level) * 31 / 100) as u16;
        let val = (attn << 8) | attn; // Left and Right channels
        unsafe { io::outw(nam + 0x02, val); }
    }
}

/// Toggles the mute state of the AC97 master output.
pub fn toggle_mute() {
    let muted = !IS_MUTED.load(Ordering::Relaxed);
    IS_MUTED.store(muted, Ordering::Relaxed);
    
    let nam = NAM_BASE.load(Ordering::Relaxed);
    if nam != 0 {
        let level = MASTER_VOLUME.load(Ordering::Relaxed);
        let attn = ((100 - level) * 31 / 100) as u16;
        let mut val = (attn << 8) | attn;
        if muted {
            val |= 0x8000; // Bit 15 is the Mute bit in the AC97 Master Volume register
        }
        unsafe { io::outw(nam + 0x02, val); }
    }
}

#[repr(C, packed)]
struct BufferDescriptor {
    pointer: u32,
    length: u16,
    flags: u16,
}

/// The Buffer Descriptor List (BDL) must be 8-byte aligned.
static mut BDL: [BufferDescriptor; 32] = unsafe { core::mem::zeroed() };
static mut PCM_BUFFER: [i16; 4096] = [0; 4096];

pub struct Ac97Driver;

impl pci::PciDriver for Ac97Driver {
    fn name(&self) -> &'static str { "AC97 Audio Controller" }
    
    fn device_ids(&self) -> &[(u16, u16)] {
        &[(0x8086, 0x2415)] // Intel 82801AA AC'97
    }

    fn initialize(&self, dev: &pci::PciDevice) {
        let (nam, _) = dev.get_bar(0);
        let (nabm, _) = dev.get_bar(1);
        
        NAM_BASE.store(nam as u16, Ordering::Relaxed);
        NABM_BASE.store(nabm as u16, Ordering::Relaxed);

        // Apply initial volume
        set_master_volume(80);

        let nam = NAM_BASE.load(Ordering::Relaxed);
        let nabm = NABM_BASE.load(Ordering::Relaxed);
        unsafe {
            // 1. Reset the AC97 Controller
            io::outw(nabm + 0x2C, 0x0001); // Global Control Reset

            // 2. Set PCM Out Volume (NAM registers)
            io::outw(nam + 0x18, 0x0000); // PCM Out Volume: 0 is max

            // 3. Setup BDL pointer in NABM
        let bdl_ptr = &BDL as *const _ as u32;
        io::outl(nabm + 0x10, bdl_ptr); // PCM Out Buffer Descriptor List Base Address
        
        // 4. Set "Last Valid Index" to 0 for a single buffer play for now
        io::outb(nabm + 0x15, 0); 
        }
        crate::serial_println!("[AUDIO] AC97 Ready at NAM:{:#x} NABM:{:#x}", nam, nabm);
    }

    fn detach(&self, _device: &pci::PciDevice) {
        let nabm = NABM_BASE.swap(0, Ordering::Relaxed);
        NAM_BASE.store(0, Ordering::Relaxed);

        if nabm != 0 {
            unsafe {
                // 1. Stop PCM Output DMA engine (Run bit)
                let ctrl = io::inb(nabm + 0x1B);
                io::outb(nabm + 0x1B, ctrl & !0x01);

                // 2. Clear any pending status bits
                io::outw(nabm + 0x16, 0x001C);
            }
        }
        crate::serial_println!("[AUDIO] AC97 Controller Detached Safely.");
    }
}

pub static AC97_DRIVER: Ac97Driver = Ac97Driver;

/// Plays a pleasant C-Major chime upon system startup.
pub fn play_startup_chime() {
    let nabm = NABM_BASE.load(Ordering::Relaxed);
    if nabm == 0 {
        return;
    }

    unsafe {
        // Fill the PCM buffer with an additive C-Major triad chord (C5, E5, G5)
        // At 44100Hz, periods of 84, 66, and 56 samples approximate these notes.
        for i in 0..4096 {
            let v1 = if (i % 84) < 42 { 1000 } else { -1000 }; // ~523Hz (C5)
            let v2 = if (i % 66) < 33 { 1000 } else { -1000 }; // ~668Hz (E5)
            let v3 = if (i % 56) < 28 { 1000 } else { -1000 }; // ~787Hz (G5)
            PCM_BUFFER[i] = v1 + v2 + v3;
        }

        BDL[0].pointer = &PCM_BUFFER as *const _ as u32;
        BDL[0].length = 4096;
        BDL[0].flags = 0x8000; // IOC

        io::outb(nabm + 0x15, 0); // Last Valid Index
        let ctrl = io::inb(nabm + 0x1B);
        io::outb(nabm + 0x1B, ctrl | 0x01); // Run
    }
}

pub fn play_tone(frequency: u32) {
    let nabm = NABM_BASE.load(Ordering::Relaxed);
    if frequency == 0 || nabm == 0 {
        stop_tone();
        return;
    }

    // Simulate a "beep" by filling a buffer with a square/sine wave
    // AC97 typically defaults to 44100Hz
    let sample_rate = 44100;
    let period = if frequency > 0 { sample_rate / frequency } else { 1 };

    unsafe {
        for i in 0..4096 {
            PCM_BUFFER[i] = if (i as u32 / (period / 2)) % 2 == 0 { 8000 } else { -8000 };
        }

        // Prepare BDL entry
        BDL[0].pointer = &PCM_BUFFER as *const _ as u32;
        BDL[0].length = 4096; // Number of samples
        BDL[0].flags = 0x8000; // Interrupt on completion (IOC)

        let nabm = NABM_BASE.load(Ordering::Relaxed);
        
        // Start Playback (PCM Out Control Register)
        // Set bit 0 (Run/Pause)
        let ctrl = io::inb(nabm + 0x1B);
        io::outb(nabm + 0x1B, ctrl | 0x01);
    }
}

/// Plays a raw PCM buffer.
pub fn play_pcm(samples: &[i16]) {
    let nabm = NABM_BASE.load(Ordering::Relaxed);
    if nabm == 0 { return; }
    unsafe {
        // Simple implementation: use first descriptor
        BDL[0].pointer = samples.as_ptr() as u32;
        BDL[0].length = (samples.len() & 0xFFFF) as u16;
        BDL[0].flags = 0x8000;

        io::outb(nabm + 0x15, 0); // Last Valid Index = 0
        let ctrl = io::inb(nabm + 0x1B);
        io::outb(nabm + 0x1B, ctrl | 0x01);
    }
}

pub fn stop_tone() {
    let nabm = NABM_BASE.load(Ordering::Relaxed);
    if nabm == 0 { return; }
    unsafe {
        let ctrl = io::inb(nabm + 0x1B);
        io::outb(nabm + 0x1B, ctrl & !0x01); // Stop Run bit
    }
}

/// Helper to output a long to I/O port
mod io_ext {
    pub unsafe fn outl(port: u16, val: u32) {
        core::arch::asm!("out dx, eax", in("dx") port, in("eax") val);
    }
    pub unsafe fn inl(port: u16) -> u32 {
        let val: u32;
        core::arch::asm!("in eax, dx", out("eax") val, in("dx") port);
        val
    }
}