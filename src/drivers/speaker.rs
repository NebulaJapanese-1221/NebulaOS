//! Driver for AC97 Audio Controller (Actual Speaker/Line Out).

use crate::kernel::io;
use crate::kernel::paging::allocate_frame;
use spin::Mutex;
use core::ptr;

/// The Mixer Base Address (typically BAR0).
const DEFAULT_AC97_MIXER: u16 = 0x1000; 
/// The Bus Master Base Address (typically BAR1).
const DEFAULT_AC97_BUS_MASTER: u16 = 0x1100;

/// AC97 Mixer Registers
const REG_MASTER_VOLUME: u16 = 0x02;
const REG_PCM_OUT_VOLUME: u16 = 0x18;

/// AC97 Bus Master Registers (Offsets from BAR1)
const PCM_OUT_BDBAR: u16 = 0x10; // Buffer Descriptor Base Address
const PCM_OUT_CIV:   u16 = 0x14; // Current Index Value
const PCM_OUT_LVI:   u16 = 0x15; // Last Valid Index
const PCM_OUT_SR:    u16 = 0x16; // Status Register
const PCM_OUT_CR:    u16 = 0x1B; // Control Register

/// Buffer Descriptor structure for AC97 DMA.
#[repr(C, packed)]
struct BufferDescriptor {
    pointer: u32,
    header: u16, // Length in samples and control bits
}

const BDL_ENTRY_IOC: u16 = 1 << 15; // Interrupt on Completion
const BDL_ENTRY_BUP: u16 = 1 << 14; // Buffer Underrun Policy

pub struct SpeakerDriver {
    pub mixer_base: u16,
    pub bus_master_base: u16,
    pub master_volume: u8, // 0 to 63 (0 is max, 63 is mute in hardware)
    pub muted: bool,
    bdl_phys: u32,
}

impl SpeakerDriver {
    pub const fn new() -> Self {
        Self {
            mixer_base: DEFAULT_AC97_MIXER,
            bus_master_base: DEFAULT_AC97_BUS_MASTER,
            master_volume: 20, // Default to a reasonable volume
            muted: false,
            bdl_phys: 0,
        }
    }

    /// Initializes the AC97 controller. Supports dynamic base addresses
    /// for compatibility with different PCI hardware brands (Intel, VIA, Realtek).
    pub fn init(&mut self, mixer_port: Option<u16>, bus_master_port: Option<u16>) {
        // Auto-detect if no specific ports provided
        if mixer_port.is_none() || bus_master_port.is_none() {
            if let Some((mixer, bus_master)) = crate::drivers::pci::find_ac97_device() {
                self.mixer_base = mixer;
                self.bus_master_base = bus_master;
            }
        } else {
            if let Some(port) = mixer_port { self.mixer_base = port; }
            if let Some(port) = bus_master_port { self.bus_master_base = port; }
        }

        // Reset the AC97 codec
        unsafe {
            io::outw(self.mixer_base + 0x00, 0x0000);
            io::wait();
        }

        // Allocate a physical page for the Buffer Descriptor List (BDL)
        if let Some(frame) = allocate_frame() {
            self.bdl_phys = frame as u32;
        }

        self.set_volume(31); // 50% Volume
    }

    /// Sets the master volume (0 to 100).
    pub fn set_volume(&mut self, percent: u8) {
        let percent = percent.min(100);
        // AC97 volume is inverted: 0 is loudest, 63 is quietest.
        // We map 0-100% to 63-0.
        let vol_value = 63 - (percent * 63 / 100);
        self.master_volume = vol_value;

        self.update_hardware_volume();
    }

    /// Starts playing PCM audio from a buffer using DMA.
    /// This implementation assumes a single buffer loop.
    pub unsafe fn play_pcm(&self, buffer_phys: u32, samples: u16) {
        if self.bdl_phys == 0 { return; }

        let bdl = self.bdl_phys as *mut BufferDescriptor;
        
        // Setup the first descriptor to point to our audio data
        // Header contains length (samples) and flags
        ptr::write_volatile(bdl, BufferDescriptor {
            pointer: buffer_phys,
            header: samples | BDL_ENTRY_IOC, // Interrupt when this buffer is finished
        });

        // 1. Reset PCM Out channel
        let mut cr = io::inb(self.bus_master_base + PCM_OUT_CR);
        io::outb(self.bus_master_base + PCM_OUT_CR, cr | 0x02); // Reset bit
        
        // 2. Set the physical address of the BDL
        io::outl(self.bus_master_base + PCM_OUT_BDBAR, self.bdl_phys);
        
        // 3. Set Last Valid Index (LVI) to 0 (we only have one descriptor)
        io::outb(self.bus_master_base + PCM_OUT_LVI, 0);
        
        // 4. Start playback (Run bit)
        cr = io::inb(self.bus_master_base + PCM_OUT_CR);
        io::outb(self.bus_master_base + PCM_OUT_CR, cr | 0x01 | 0x08); // Run + IOC Enable
    }

    /// Plays a short, generated startup "ping" sound.
    pub fn play_startup_sound(&self) {
        if self.bdl_phys == 0 { return; }
        
        // Generate a simple 440Hz pulse for 200ms
        if let Some(frame) = allocate_frame() {
            let buffer = frame as *mut i16;
            unsafe {
                for i in 0..8000 {
                    // Simple square wave pulse
                    let val = if (i / 50) % 2 == 0 { 4000 } else { -4000 };
                    ptr::write_volatile(buffer.add(i), val);
                }
                self.play_pcm(frame as u32, 8000);
            }
        }
    }

    pub fn stop_pcm(&self) {
        let cr = unsafe { io::inb(self.bus_master_base + PCM_OUT_CR) };
        unsafe { io::outb(self.bus_master_base + PCM_OUT_CR, cr & !0x01); }
    }

    pub fn mute(&mut self) {
        self.muted = true;
        self.update_hardware_volume();
    }

    pub fn unmute(&mut self) {
        self.muted = false;
        self.update_hardware_volume();
    }

    fn update_hardware_volume(&self) {
        // Register format: [15] Mute bit, [13:8] Left Volume, [5:0] Right Volume
        let mut val: u16 = ((self.master_volume as u16) << 8) | (self.master_volume as u16);
        
        if self.muted {
            val |= 0x8000;
        }

        unsafe {
            io::outw(self.mixer_base + REG_MASTER_VOLUME, val);
            io::outw(self.mixer_base + REG_PCM_OUT_VOLUME, val);
        }
    }
}

pub static SPEAKER: Mutex<SpeakerDriver> = Mutex::new(SpeakerDriver::new());

/// Public API for volume control used by the GUI or Shell.
pub fn set_master_volume(percent: u8) {
    let mut speaker = SPEAKER.lock();
    speaker.set_volume(percent);
}