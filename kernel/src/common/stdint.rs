pub type u8 = core::ffi::c_uchar;
pub type u16 = core::ffi::c_ushort;
pub type u32 = core::ffi::c_uint;
pub type u64 = core::ffi::c_ulonglong;

pub type i8 = core::ffi::c_schar;
pub type i16 = core::ffi::c_short;
pub type i32 = core::ffi::c_int;
pub type i64 = core::ffi::c_longlong;

pub type usize = u32;
pub type isize = i32;
pub type uintptr_t = u32;
pub type intptr_t = i32;
pub type size_t = u32;

pub const NULL: *mut core::ffi::c_void = 0 as *mut core::ffi::c_void;

pub const UINT8_MAX: u8 = 255;
pub const UINT16_MAX: u16 = 65535;
pub const UINT32_MAX: u32 = 4294967295;
pub const UINT64_MAX: u64 = 18446744073709551615;

pub const INT8_MAX: i8 = 127;
pub const INT16_MAX: i16 = 32767;
pub const INT32_MAX: i32 = 2147483647;
pub const INT64_MAX: i64 = 9223372036854775807;
