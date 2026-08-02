use crate::serial_println;
use crate::services;
use alloc::vec;

// Service-related system calls
const SYS_NETWORK_SOCKET: u32 = 40;
const SYS_NETWORK_BIND: u32 = 41;
const SYS_NETWORK_CONNECT: u32 = 42;
const SYS_NETWORK_SEND: u32 = 43;
const SYS_NETWORK_RECEIVE: u32 = 44;
const SYS_NETWORK_CLOSE: u32 = 45;

const SYS_SECURITY_AUTHENTICATE: u32 = 50;
const SYS_SECURITY_GET_UID: u32 = 51;
const SYS_SECURITY_CHECK_PERMISSION: u32 = 52;

const SYS_POWER_GET_CPU_FREQ: u32 = 60;
const SYS_POWER_SET_CPU_FREQ: u32 = 61;
const SYS_POWER_GET_BATTERY: u32 = 62;
const SYS_POWER_GET_THERMAL: u32 = 63;

const SYS_LOADER_LIST_APPS: u32 = 70;
const SYS_LOADER_LAUNCH_APP: u32 = 71;
const SYS_LOADER_REGISTER_APP: u32 = 72;

#[cfg(target_arch = "x86")]
pub type Register = u32;
#[cfg(target_arch = "x86_64")]
pub type Register = u64;

#[cfg(target_arch = "x86")]
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct SyscallRegisters {
    pub gs: u32, pub fs: u32, pub es: u32, pub ds: u32,
    pub edi: u32, pub esi: u32, pub ebp: u32, pub kernel_esp: u32, 
    pub ebx: u32, pub edx: u32, pub ecx: u32, pub eax: u32,
    pub eip: u32, pub cs: u32, pub eflags: u32,
    pub esp: u32,
    pub ss: u32,
}

#[cfg(target_arch = "x86_64")]
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct SyscallRegisters {
    pub gs: u64, pub fs: u64, pub es: u64, pub ds: u64,
    pub r15: u64, pub r14: u64, pub r13: u64, pub r12: u64,
    pub r11: u64, pub r10: u64, pub r9: u64, pub r8: u64,
    pub rdi: u64, pub rsi: u64, pub rbp: u64, pub rbx: u64,
    pub rdx: u64, pub rcx: u64, pub rax: u64,
    pub rip: u64, pub cs: u64, pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl SyscallRegisters {
    pub fn is_user(&self) -> bool {
        (self.cs & 0x3) == 3 
    }

    #[allow(dead_code)]
    #[cfg(target_arch = "x86")]
    pub fn get_user_esp(&self) -> u32 {
        if self.is_user() { self.esp } else { self.kernel_esp }
    }

    #[allow(dead_code)]
    #[cfg(target_arch = "x86_64")]
    pub fn get_user_esp(&self) -> u64 {
        if self.is_user() { self.rsp } else { 0 }
    }
}

#[cfg(target_arch = "x86")]
#[inline(always)]
pub fn syscall_arg1(regs: &SyscallRegisters) -> u32 { regs.ebx }

#[cfg(target_arch = "x86")]
#[inline(always)]
pub fn syscall_arg2(regs: &SyscallRegisters) -> u32 { regs.ecx }

#[cfg(target_arch = "x86")]
#[inline(always)]
pub fn syscall_arg3(regs: &SyscallRegisters) -> u32 { regs.edx }

#[cfg(target_arch = "x86")] 
#[inline(always)]
pub fn syscall_get_id(regs: &SyscallRegisters) -> u32 { regs.eax }

#[cfg(target_arch = "x86")] 
#[inline(always)]
pub fn syscall_set_return(regs: &mut SyscallRegisters, value: u32) {
    regs.eax = value;
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn syscall_arg1(regs: &SyscallRegisters) -> u32 { regs.rdi as u32 }

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn syscall_arg2(regs: &SyscallRegisters) -> u32 { regs.rsi as u32 }

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn syscall_arg3(regs: &SyscallRegisters) -> u32 { regs.rdx as u32 }

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn syscall_get_id(regs: &SyscallRegisters) -> u32 { regs.rax as u32 }

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn syscall_set_return(regs: &mut SyscallRegisters, value: u32) {
    regs.rax = value as u64;
}

#[cfg(target_arch = "x86")]
#[inline(always)]
pub fn syscall_set_time(regs: &mut SyscallRegisters, hour: u32, minute: u32, second: u32) {
    regs.ebx = hour;
    regs.ecx = minute;
    regs.edx = second;
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn syscall_set_time(regs: &mut SyscallRegisters, hour: u32, minute: u32, second: u32) {
    regs.rdi = hour as u64;
    regs.rsi = minute as u64;
    regs.rdx = second as u64;
}

pub fn syscall_handler_rust(regs_ptr: &mut SyscallRegisters) -> u32 {
    let mut regs = *regs_ptr;
    let eax = syscall_get_id(&regs);

    if eax != 0 && eax != 1 && eax != 2 && eax != 3 && eax != 4 && eax != 5 && eax != 6 {
        serial_println!("DEBUG SYSCALL: ID={} (User={})", eax, regs.is_user());
    }

    let mut return_val = regs_ptr as *mut _ as u32;

    match eax {
        0 => { // Syscall 0: Yield
            return_val = crate::scheduler::schedule(regs_ptr as *mut _ as u32);
        },
        1 => { // Syscall 1: Print to Serial (Kernel only for now)
            serial_println!("Syscall: Kernel received request to print!");
        },
        2 => { // Syscall 2: Get System Time
            let time = crate::rtc::get_time();
            syscall_set_time(&mut regs, time.hour as u32, time.minute as u32, time.second as u32);
        },
        3 => { // Syscall 3: Draw Pixel
            syscall_draw_pixel(eax, syscall_arg1(&regs), syscall_arg2(&regs));
        },
        4 => { // Syscall 4: Sleep
            syscall_sleep(syscall_arg1(&regs));
        },
        5 => { // Syscall 5: Exit Process
            syscall_exit();
        },
        6 => { // Syscall 6: Spawn (Exec) New Process
            if regs.is_user() { // Only allow user mode to spawn for now
                 let entry_point = syscall_arg1(&regs) as usize;
                 let user_kernel_stack_size = 4096; // Default sizes
                 let user_stack_size = 4096 * 4; // 16KB user stack

                 let new_pid = {
                    let mut sched = crate::scheduler::get_scheduler().lock();
                    sched.spawn_user_process(entry_point as usize, user_stack_size, user_kernel_stack_size)
                 };
                 serial_println!("Spawned new user process with PID: {}", new_pid);
            }
        },
        SYS_NETWORK_SOCKET => {
            syscall_network_socket(syscall_arg1(&regs));
        },
        SYS_NETWORK_BIND => {
            syscall_network_bind(syscall_arg1(&regs), syscall_arg2(&regs), syscall_arg3(&regs));
        },
        SYS_NETWORK_CONNECT => {
            syscall_network_connect(syscall_arg1(&regs), syscall_arg2(&regs), syscall_arg3(&regs));
        },
        SYS_NETWORK_SEND => {
            syscall_network_send(syscall_arg1(&regs), syscall_arg2(&regs), syscall_arg3(&regs));
        },
        SYS_NETWORK_RECEIVE => {
            syscall_network_receive(syscall_arg1(&regs), syscall_arg2(&regs), syscall_arg3(&regs));
        },
        SYS_NETWORK_CLOSE => {
            syscall_network_close(syscall_arg1(&regs));
        },
        SYS_SECURITY_AUTHENTICATE => {
            syscall_security_authenticate(syscall_arg1(&regs), syscall_arg2(&regs));
        },
        SYS_SECURITY_GET_UID => {
            syscall_security_get_uid();
        },
        SYS_SECURITY_CHECK_PERMISSION => {
            syscall_security_check_permission(syscall_arg1(&regs), syscall_arg2(&regs));
        },
        SYS_POWER_GET_CPU_FREQ => {
            syscall_power_get_cpu_freq();
        },
        SYS_POWER_SET_CPU_FREQ => {
            syscall_power_set_cpu_freq(syscall_arg1(&regs));
        },
        SYS_POWER_GET_BATTERY => {
            syscall_power_get_battery();
        },
        SYS_POWER_GET_THERMAL => {
            syscall_power_get_thermal();
        },
        SYS_LOADER_LIST_APPS => {
            let loader = services::loader::get_loader_service().lock();
            let apps = loader.list_apps();
            // Return the number of registered apps
            syscall_set_return(&mut regs, apps.len() as u32);
        },
        SYS_LOADER_LAUNCH_APP => {
            let app_id = syscall_arg1(&regs);
            let mut loader = services::loader::get_loader_service().lock();
            match loader.launch_app(app_id) {
                Ok(pid) => {
                    syscall_set_return(&mut regs, pid);
                    serial_println!("Syscall: Launched app {} with PID {}", app_id, pid);
                }
                Err(e) => {
                    syscall_set_return(&mut regs, 0);
                    serial_println!("Syscall: Failed to launch app {}: {}", app_id, e);
                }
            }
        },
        SYS_LOADER_REGISTER_APP => {
            // In a real implementation, we'd copy the app name/path from user space
            let name_ptr = syscall_arg1(&regs);
            let _elf_ptr = syscall_arg2(&regs);
            serial_println!("Syscall: Register app request from user space (ptr=0x{:x})", name_ptr);
            syscall_set_return(&mut regs, 1); // Placeholder success
        },
        _ => {
            serial_println!("Unknown syscall: {}", eax);
        }
    }
    *regs_ptr = regs;
    return_val
}

#[cfg(target_arch = "x86")]
#[allow(dead_code)]
pub fn test_syscall() {
    unsafe {
        core::arch::asm!(
            "mov eax, 1",
            "int 0x80",
            out("eax") _,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
pub fn test_syscall() {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            inout("rax") 1u64 => _,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(target_arch = "x86")]
#[allow(dead_code)]
pub fn syscall_exec(entry_point: u32) {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 6,
            in("ebx") entry_point,
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
pub fn syscall_exec(entry_point: u32) {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") 6u64,
            in("rdi") entry_point as u64,
        );
    }
}

#[cfg(target_arch = "x86")]
#[allow(dead_code)]
pub fn syscall_sleep(ms: u32) {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 4,
            in("ebx") ms,
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
pub fn syscall_sleep(ms: u32) {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") 4u64,
            in("rdi") ms as u64,
        );
    }
}

#[cfg(target_arch = "x86")]
#[allow(dead_code)]
pub fn syscall_exit() -> ! {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 5,
            options(noreturn)
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
pub fn syscall_exit() -> ! {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") 5u64,
            options(noreturn)
        );
    }
}

#[cfg(target_arch = "x86")]
#[allow(dead_code)]
pub fn syscall_yield() {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 0,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
pub fn syscall_yield() {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") 0u64,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(target_arch = "x86")]
#[allow(dead_code)]
pub fn syscall_get_time() -> (u32, u32, u32) {
    let h: u32; let m: u32; let s: u32;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            inout("eax") 2 => _,
            out("ebx") h,
            out("ecx") m,
            out("edx") s,
            options(nostack, preserves_flags)
        );
    }
    (h, m, s)
}

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
pub fn syscall_get_time() -> (u32, u32, u32) {
    let h: u32; let m: u32; let s: u32;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            inout("rax") 2u64 => _,
            lateout("rdi") h,
            lateout("rsi") m,
            lateout("rdx") s,
            options(nostack, preserves_flags)
        );
    }
    (h, m, s)
}

#[cfg(target_arch = "x86")]
#[allow(dead_code)]
pub fn syscall_draw_pixel(x: u32, y: u32, color: u32) {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 3,
            in("ebx") x,
            in("ecx") y,
            in("edx") color,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
pub fn syscall_draw_pixel(x: u32, y: u32, color: u32) {
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") 3u64,
            in("rdi") x as u64,
            in("rsi") y as u64,
            in("rdx") color as u64,
            options(nostack, preserves_flags)
        );
    }
}

pub fn syscall_network_socket(socket_type: u32) -> u32 {
    let mut network_service = services::network::get_network_service().lock();
    match network_service.create_socket(match socket_type {
        1 => services::network::SocketType::TCP,
        2 => services::network::SocketType::UDP,
        _ => services::network::SocketType::Raw,
    }) {
        Ok(socket_id) => socket_id,
        Err(_) => 0,
    }
}

pub fn syscall_network_bind(socket_id: u32, addr: u32, port: u32) -> u32 {
    let mut network_service = services::network::get_network_service().lock();
    if let Some(socket) = network_service.get_socket_mut(socket_id) {
        socket.bind((addr, port as u16)).is_ok() as u32
    } else {
        0
    }
}

pub fn syscall_network_connect(socket_id: u32, addr: u32, port: u32) -> u32 {
    let mut network_service = services::network::get_network_service().lock();
    if let Some(socket) = network_service.get_socket_mut(socket_id) {
        socket.connect((addr, port as u16)).is_ok() as u32
    } else {
        0
    }
}

pub fn syscall_network_send(socket_id: u32, data_ptr: u32, len: u32) -> u32 {
    let mut network_service = services::network::get_network_service().lock();
    if let Some(socket) = network_service.get_socket_mut(socket_id) {
        // In a real implementation, we would copy data from user space
        let data = unsafe { core::slice::from_raw_parts(data_ptr as *const u8, len as usize) };
        socket.send(data).unwrap_or(0) as u32
    } else {
        0
    }
}

pub fn syscall_network_receive(socket_id: u32, buffer_ptr: u32, len: u32) -> u32 {
    let mut network_service = services::network::get_network_service().lock();
    if let Some(socket) = network_service.get_socket_mut(socket_id) {
        let mut buffer = vec![0u8; len as usize];
        match socket.receive(buffer.as_mut_slice()) {
            Ok(bytes_read) => {
                // In a real implementation, we would copy data to user space
                unsafe { core::ptr::copy_nonoverlapping(buffer.as_ptr(), buffer_ptr as *mut u8, bytes_read) };
                bytes_read as u32
            }
            Err(_) => 0,
        }
    } else {
        0
    }
}

pub fn syscall_network_close(socket_id: u32) -> u32 {
    let mut network_service = services::network::get_network_service().lock();
    network_service.close_socket(socket_id).is_ok() as u32
}

pub fn syscall_security_authenticate(username_ptr: u32, password_ptr: u32) -> u32 {
    let mut security_service = services::security::get_security_service().lock();

    // In a real implementation, we would copy strings from user space
    let username = unsafe { core::ffi::CStr::from_ptr(username_ptr as *const i8) };
    let password = unsafe { core::ffi::CStr::from_ptr(password_ptr as *const i8) };

    match (username.to_str(), password.to_str()) {
        (Ok(username), Ok(password)) => {
            security_service.authenticate(username, password).is_ok() as u32
        }
        _ => 0,
    }
}

pub fn syscall_security_get_uid() -> u32 {
    let security_service = services::security::get_security_service().lock();
    security_service.current_user().map(|u| u.uid).unwrap_or(0)
}

pub fn syscall_security_check_permission(uid: u32, permission: u32) -> u32 {
    let security_service = services::security::get_security_service().lock();
    let permission = match permission {
        1 => services::security::Permission::Read,
        2 => services::security::Permission::Write,
        3 => services::security::Permission::Execute,
        4 => services::security::Permission::Admin,
        _ => return 0,
    };
    security_service.check_permission(uid, permission) as u32
}

pub fn syscall_power_get_cpu_freq() -> u32 {
    let power_service = services::power::get_power_service().lock();
    power_service.get_cpu_frequency()
}

pub fn syscall_power_set_cpu_freq(freq: u32) -> u32 {
    let mut power_service = services::power::get_power_service().lock();
    power_service.set_cpu_frequency(freq).is_ok() as u32
}

pub fn syscall_power_get_battery() -> u32 {
    // Return battery percentage
    let power_service = services::power::get_power_service().lock();
    power_service.get_battery_status().capacity as u32
}

pub fn syscall_power_get_thermal() -> u32 {
    // Return CPU temperature in Celsius
    let power_service = services::power::get_power_service().lock();
    power_service.get_thermal_status().cpu_temp as u32
}
