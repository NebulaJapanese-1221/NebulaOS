//! Driver for battery status monitoring via ACPI.

use spin::Mutex;
use core::sync::atomic::{AtomicU8, AtomicBool, Ordering};
use core::sync::atomic::AtomicU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryState {
    Discharging,
    Charging,
    Full,
}

pub struct BatteryInfo {
    pub percentage: u8,
    pub health: u8,
    pub cycle_count: u32,
    pub state: BatteryState,
}

/// Global state for the battery.
static BATTERY_LEVEL: AtomicU8 = AtomicU8::new(100);
static IS_CHARGING: AtomicBool = AtomicBool::new(false);
static DESIGN_CAPACITY: AtomicU32 = AtomicU32::new(1000); // mWh
static FULL_CHARGE_CAPACITY: AtomicU32 = AtomicU32::new(1000);
static CYCLE_COUNT: AtomicU32 = AtomicU32::new(0);
static REMAINING_CAPACITY: AtomicU32 = AtomicU32::new(1000);

pub struct BatteryDriver;

impl BatteryDriver {
    pub fn update_status(&self, remaining: u32, charging: bool, design: Option<u32>, full: Option<u32>, cycles: Option<u32>) {
        if let Some(d) = design { DESIGN_CAPACITY.store(d, Ordering::Relaxed); }
        if let Some(f) = full { FULL_CHARGE_CAPACITY.store(f, Ordering::Relaxed); }
        if let Some(c) = cycles { CYCLE_COUNT.store(c, Ordering::Relaxed); }
        REMAINING_CAPACITY.store(remaining, Ordering::Relaxed);
        IS_CHARGING.store(charging, Ordering::Relaxed);
        
        let design = DESIGN_CAPACITY.load(Ordering::Relaxed);
        let percentage = ((remaining * 100) / design.max(1)).min(100) as u8;
        BATTERY_LEVEL.store(percentage, Ordering::Relaxed);
    }

    pub fn get_info(&self) -> BatteryInfo {
        let level = BATTERY_LEVEL.load(Ordering::Relaxed);
        let charging = IS_CHARGING.load(Ordering::Relaxed);
        let design = DESIGN_CAPACITY.load(Ordering::Relaxed);
        let full = FULL_CHARGE_CAPACITY.load(Ordering::Relaxed);
        let cycle_count = CYCLE_COUNT.load(Ordering::Relaxed);

        let health = if design > 0 {
            ((full * 100) / design).min(100) as u8
        } else {
            100
        };
        
        let state = if level >= 100 && !charging {
            BatteryState::Full
        } else if charging {
            BatteryState::Charging
        } else {
            BatteryState::Discharging
        };

        BatteryInfo { percentage: level, health, cycle_count, state }
    }
}

pub static BATTERY: Mutex<BatteryDriver> = Mutex::new(BatteryDriver);