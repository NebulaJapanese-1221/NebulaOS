use crate::common::stdint::*;

static mut ACPI_INITIALIZED: bool = false;

#[no_mangle]
pub unsafe extern "C" fn acpi_init() -> bool {
    ACPI_INITIALIZED = true;
    true
}

pub unsafe fn acpi_get_table(_signature: *const u8) -> *mut u8 {
    core::ptr::null_mut()
}

pub unsafe fn acpi_enumerate_tables() {}

pub unsafe fn acpi_parse_madt() {}
