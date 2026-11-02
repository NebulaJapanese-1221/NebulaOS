//! Rust wrappers for the system-call ABI available to Ring-3 processes.

pub const SYSCALL_PRINT: u32 = 0;
pub const SYSCALL_EXIT: u32 = 2;
pub const SYSCALL_GET_TICKS: u32 = 3;
pub const SYSCALL_DRAW_FILL_RECT: u32 = 4;
pub const SYSCALL_DRAW_RECT: u32 = 5;
pub const SYSCALL_DRAW_TEXT: u32 = 6;
pub const SYSCALL_POLL_EVENT: u32 = 7;
pub const SYSCALL_GET_APP_CONTEXT: u32 = 8;
pub const SYSCALL_STORAGE_LIST: u32 = 9;
pub const SYSCALL_STORAGE_READ: u32 = 10;
pub const SYSCALL_STORAGE_WRITE: u32 = 11;
pub const SYSCALL_STORAGE_DELETE: u32 = 12;
pub const SYSCALL_STORAGE_RENAME: u32 = 13;
pub const SYSCALL_STORAGE_FORMAT: u32 = 14;
pub const SYSCALL_SYSTEM_STATS: u32 = 15;
pub const SYSCALL_SET_WALLPAPER_THEME: u32 = 16;
pub const SYSCALL_SET_MOUSE_SENSITIVITY: u32 = 17;
pub const MAX_WRITE_LENGTH: usize = 256;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DrawCommand {
    pub kind: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub color: u32,
    pub scale: u32,
    pub text_ptr: u32,
    pub text_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AppEvent {
    pub kind: u32,
    pub key: u32,
    pub x: u32,
    pub y: u32,
    pub left_down: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct AppContext {
    pub app_id: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub dark_theme: u32,
    pub mouse_sensitivity: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct StorageRequest {
    pub op: u32,
    pub name_ptr: u32,
    pub name_len: u32,
    pub data_ptr: u32,
    pub data_len: u32,
    pub out_ptr: u32,
    pub out_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct StorageFileInfo {
    pub name: [u8; 16],
    pub length: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SystemStats {
    pub ticks: u32,
    pub free_frames: u32,
    pub total_frames: u32,
}

#[cfg(feature = "user-app")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyscallError {
    MessageTooLong,
    Unavailable,
    InvalidArgument,
}

#[cfg(feature = "user-app")]
pub fn write(message: &[u8]) -> Result<(), SyscallError> {
    if message.len() > MAX_WRITE_LENGTH {
        return Err(SyscallError::MessageTooLong);
    }

    let result = invoke(SYSCALL_PRINT, message.as_ptr() as u32, message.len() as u32);
    if result == message.len() as u32 {
        Ok(())
    } else {
        Err(SyscallError::Unavailable)
    }
}

#[cfg(feature = "user-app")]
pub fn uptime_ticks() -> Result<u32, SyscallError> {
    let ticks = invoke(SYSCALL_GET_TICKS, 0, 0);
    if ticks == u32::MAX {
        Err(SyscallError::Unavailable)
    } else {
        Ok(ticks)
    }
}

#[cfg(feature = "user-app")]
pub fn exit(code: u32) -> Result<(), SyscallError> {
    if invoke(SYSCALL_EXIT, code, 0) == u32::MAX {
        Err(SyscallError::Unavailable)
    } else {
        Ok(())
    }
}

#[cfg(feature = "user-app")]
pub fn draw(command: &mut DrawCommand) -> Result<(), SyscallError> {
    let number = match command.kind {
        0 => SYSCALL_DRAW_FILL_RECT,
        1 => SYSCALL_DRAW_RECT,
        2 => SYSCALL_DRAW_TEXT,
        _ => return Err(SyscallError::InvalidArgument),
    };
    call_struct(number, command)
}

#[cfg(feature = "user-app")]
pub fn poll_event(event: &mut AppEvent) -> Result<bool, SyscallError> {
    let result = invoke(SYSCALL_POLL_EVENT, event as *mut AppEvent as u32, 0);
    match result {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(SyscallError::Unavailable),
    }
}

#[cfg(feature = "user-app")]
pub fn app_context(context: &mut AppContext) -> Result<(), SyscallError> {
    if invoke(
        SYSCALL_GET_APP_CONTEXT,
        context as *mut AppContext as u32,
        0,
    ) == 0
    {
        Ok(())
    } else {
        Err(SyscallError::Unavailable)
    }
}

#[cfg(feature = "user-app")]
pub fn storage(request: &mut StorageRequest) -> u32 {
    let number = match request.op {
        0 => SYSCALL_STORAGE_LIST,
        1 => SYSCALL_STORAGE_READ,
        2 => SYSCALL_STORAGE_WRITE,
        3 => SYSCALL_STORAGE_DELETE,
        4 => SYSCALL_STORAGE_RENAME,
        5 => SYSCALL_STORAGE_FORMAT,
        _ => return u32::MAX,
    };
    invoke(number, request as *mut StorageRequest as u32, 0)
}

#[cfg(feature = "user-app")]
pub fn system_stats(stats: &mut SystemStats) -> Result<(), SyscallError> {
    call_struct(SYSCALL_SYSTEM_STATS, stats)
}

#[cfg(feature = "user-app")]
pub fn set_dark_theme(enabled: bool) -> Result<(), SyscallError> {
    if invoke(SYSCALL_SET_WALLPAPER_THEME, u32::from(enabled), 0) == 0 {
        Ok(())
    } else {
        Err(SyscallError::Unavailable)
    }
}

#[cfg(feature = "user-app")]
pub fn set_mouse_sensitivity(sensitivity: u32) -> Result<(), SyscallError> {
    if !matches!(sensitivity, 1 | 2 | 4) {
        return Err(SyscallError::InvalidArgument);
    }
    if invoke(SYSCALL_SET_MOUSE_SENSITIVITY, sensitivity, 0) == 0 {
        Ok(())
    } else {
        Err(SyscallError::Unavailable)
    }
}

#[cfg(feature = "user-app")]
fn call_struct<T>(number: u32, value: &mut T) -> Result<(), SyscallError> {
    if invoke(number, value as *mut T as u32, 0) == 0 {
        Ok(())
    } else {
        Err(SyscallError::Unavailable)
    }
}

#[cfg(feature = "user-app")]
#[inline]
fn invoke(number: u32, first_argument: u32, second_argument: u32) -> u32 {
    let mut result = number;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            inout("eax") result,
            in("ebx") first_argument,
            in("ecx") second_argument,
        );
    }
    result
}
