//! Driver for screen brightness control.
//! Currently simulates brightness control and provides an interface for future ACPI integration.

use spin::Mutex;
use core::sync::atomic::{AtomicU8, AtomicBool, Ordering};

/// Global atomic to store the current brightness level (0-100).
pub static BRIGHTNESS_LEVEL: AtomicU8 = AtomicU8::new(100);
/// Flag used to notify the GUI that an OSD update is required.
pub static BRIGHTNESS_UPDATED: AtomicBool = AtomicBool::new(false);

pub struct BrightnessDriver {
    // Placeholder for ACPI related data if needed in the future
    // pub acpi_method_path: Option<String>,
}

impl BrightnessDriver {
    pub const fn new() -> Self {
        Self {}
    }

    /// Initializes the brightness driver.
    /// In a real system, this would involve detecting ACPI backlight methods.
    pub fn init(&mut self) {
        // For now, just set a default.
        BRIGHTNESS_LEVEL.store(100, Ordering::Relaxed);
    }

    /// Sets the brightness level (0-100).
    pub fn set_brightness(&mut self, level: u8) {
        let clamped_level = level.clamp(0, 100);
        BRIGHTNESS_LEVEL.store(clamped_level, Ordering::Relaxed);
        BRIGHTNESS_UPDATED.store(true, Ordering::Relaxed);
        
        // In a real implementation, this would involve calling an ACPI method.
        crate::kernel::acpi::acpi_set_backlight_level(clamped_level);
    }

    /// Adjusts brightness by a relative percentage (e.g., +5 or -5).
    pub fn increment_brightness(&mut self, delta: i8) {
        let current_level = BRIGHTNESS_LEVEL.load(Ordering::Relaxed) as i16;
        let new_level = (current_level + delta as i16).clamp(0, 100) as u8;
        self.set_brightness(new_level);
    }
}

pub static BRIGHTNESS: Mutex<BrightnessDriver> = Mutex::new(BrightnessDriver::new());

/// Public API for brightness control used by the GUI or Shell.
pub fn increment_master_brightness(delta: i8) {
    let mut brightness = BRIGHTNESS.lock();
    brightness.increment_brightness(delta);
}

/// Public API to set brightness to a specific value.
pub fn set_master_brightness(level: u8) {
    let mut brightness = BRIGHTNESS.lock();
    brightness.set_brightness(level);
}