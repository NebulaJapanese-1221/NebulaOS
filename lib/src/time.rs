use crate::common::stdint::*;

static mut TIMER_TICKS: u32 = 0;

pub unsafe fn sleep(_seconds: i32) {}

pub unsafe fn usleep(_microseconds: u32) {}

pub unsafe fn time(_timer: *mut u64) -> u64 {
    TIMER_TICKS as u64
}

pub unsafe fn clock() -> u64 {
    TIMER_TICKS as u64
}

pub unsafe fn timer_set_ticks(ticks: u32) {
    TIMER_TICKS = ticks;
}
