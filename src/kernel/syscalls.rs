use crate::serial_println;
use crate::process::ProcessState;

// Service-related system calls
const SYS_NETWORK_SOCKET: usize = 40;
const SYS_NETWORK_BIND: usize = 41;
const SYS_NETWORK_CONNECT: usize = 42;
const SYS_NETWORK_SEND: usize = 43;
const SYS_NETWORK_RECEIVE: usize = 44;
const SYS_NETWORK_CLOSE: usize = 45;

const SYS_SECURITY_AUTHENTICATE: usize = 50;
const SYS_SECURITY_GET_UID: usize = 51;
const SYS_SECURITY_CHECK_PERMISSION: usize = 52;

const SYS_POWER_GET_CPU_FREQ: usize = 60;
const SYS_POWER_SET_CPU_FREQ: usize = 61;
const SYS_POWER_GET_BATTERY: usize = 62;
const SYS_POWER_GET_THERMAL: usize = 63;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct SyscallRegisters {
    pub gs: u32, pub fs: u32, pub es: u32, pub ds: u32,
    pub edi: u32, pub esi: u32, pub ebp: u32, pub kernel_esp: u32, 
    pub ebx: u32, pub edx: u32, pub ecx: u32, pub eax: u32,
    pub eip: u32, pub cs: u32, pub eflags: u32,
    pub esp: u32, // User-mode stack pointer
    pub ss: u32,  // User-mode segment selector
}

impl SyscallRegisters {
    pub fn is_user(&self) -> bool {
        (self.cs & 0x3) == 3 
    }

    #[allow(dead_code)]
    pub fn get_user_esp(&self) -> u32 {
        if self.is_user() {
            self.esp 
        } else {
            self.kernel_esp 
        }
    }
}

pub fn syscall_handler_rust(regs_ptr: &mut SyscallRegisters) -> u32 {
    let mut regs = *regs_ptr; 
    let eax = regs.eax;

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
            regs.ebx = time.hour as u32;
            regs.ecx = time.minute as u32;
            regs.edx = time.second as u32;
        },
        3 => { // Syscall 3: Draw Pixel
            syscall_draw_pixel(regs.eax, regs.ebx, regs.ecx);
        },
        4 => { // Syscall 4: Sleep
            syscall_sleep(regs.eax);
        },
        5 => { // Syscall 5: Exit Process
            syscall_exit();
        },
        6 => { // Syscall 6: Spawn (Exec) New Process
            if regs.is_user() { // Only allow user mode to spawn for now
                 let entry_point = regs.ebx;
                 let user_kernel_stack_size = 4096; // Default sizes
                 let user_stack_size = 4096 * 4; // 16KB user stack

                 let new_pid = {
                    let mut sched = crate::scheduler::SCHEDULER.lock();
                    sched.spawn_user_process(entry_point, user_stack_size, user_kernel_stack_size)
                 };
                 serial_println!("Spawned new user process with PID: {}", new_pid);
            }
        },
        SYS_NETWORK_SOCKET => {
            syscall_network_socket(regs.eax);
        },
        SYS_NETWORK_BIND => {
            syscall_network_bind(regs.eax, regs.ebx, regs.ecx);
        },
        SYS_NETWORK_CONNECT => {
            syscall_network_connect(regs.eax, regs.ebx, regs.ecx);
        },
        SYS_NETWORK_SEND => {
            syscall_network_send(regs.eax, regs.ebx, regs.ecx);
        },
        SYS_NETWORK_RECEIVE => {
            syscall_network_receive(regs.eax, regs.ebx, regs.ecx);
        },
        SYS_NETWORK_CLOSE => {
            syscall_network_close(regs.eax);
        },
        SYS_SECURITY_AUTHENTICATE => {
            syscall_security_authenticate(regs.eax, regs.ebx);
        },
        SYS_SECURITY_GET_UID => {
            syscall_security_get_uid();
        },
        SYS_SECURITY_CHECK_PERMISSION => {
            syscall_security_check_permission(regs.eax, regs.ebx);
        },
        SYS_POWER_GET_CPU_FREQ => {
            syscall_power_get_cpu_freq();
        },
        SYS_POWER_SET_CPU_FREQ => {
            syscall_power_set_cpu_freq(regs.eax);
        },
        SYS_POWER_GET_BATTERY => {
            syscall_power_get_battery();
        },
        SYS_POWER_GET_THERMAL => {
            syscall_power_get_thermal();
        },
        _ => {
            serial_println!("Unknown syscall: {}", eax);
        }
    }
    *regs_ptr = regs;
    return_val
}

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

pub fn syscall_network_socket(socket_type: u32) -> u32 {
    let mut network_service = services::network::NETWORK_SERVICE.lock();
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
    let mut network_service = services::network::NETWORK_SERVICE.lock();
    if let Some(socket) = network_service.get_socket_mut(socket_id) {
        socket.bind((addr, port)).is_ok() as u32
    } else {
        0
    }
}

pub fn syscall_network_connect(socket_id: u32, addr: u32, port: u32) -> u32 {
    let mut network_service = services::network::NETWORK_SERVICE.lock();
    if let Some(socket) = network_service.get_socket_mut(socket_id) {
        socket.connect((addr, port)).is_ok() as u32
    } else {
        0
    }
}

pub fn syscall_network_send(socket_id: u32, data_ptr: u32, len: u32) -> u32 {
    let mut network_service = services::network::NETWORK_SERVICE.lock();
    if let Some(socket) = network_service.get_socket_mut(socket_id) {
        // In a real implementation, we would copy data from user space
        let data = unsafe { core::slice::from_raw_parts(data_ptr as *const u8, len as usize) };
        socket.send(data).unwrap_or(0)
    } else {
        0
    }
}

pub fn syscall_network_receive(socket_id: u32, buffer_ptr: u32, len: u32) -> u32 {
    let mut network_service = services::network::NETWORK_SERVICE.lock();
    if let Some(socket) = network_service.get_socket_mut(socket_id) {
        let mut buffer = vec![0; len as usize];
        match socket.receive(&mut buffer) {
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
    let mut network_service = services::network::NETWORK_SERVICE.lock();
    network_service.close_socket(socket_id).is_ok() as u32
}

pub fn syscall_security_authenticate(username_ptr: u32, password_ptr: u32) -> u32 {
    let mut security_service = services::security::SECURITY_SERVICE.lock();

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
    let security_service = services::security::SECURITY_SERVICE.lock();
    security_service.current_user().map(|u| u.uid).unwrap_or(0)
}

pub fn syscall_security_check_permission(uid: u32, permission: u32) -> u32 {
    let security_service = services::security::SECURITY_SERVICE.lock();
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
    let power_service = services::power::POWER_SERVICE.lock();
    power_service.get_cpu_frequency()
}

pub fn syscall_power_set_cpu_freq(freq: u32) -> u32 {
    let mut power_service = services::power::POWER_SERVICE.lock();
    power_service.set_cpu_frequency(freq).is_ok() as u32
}

pub fn syscall_power_get_battery() -> u32 {
    // Return battery percentage
    let power_service = services::power::POWER_SERVICE.lock();
    power_service.get_battery_status().capacity as u32
}

pub fn syscall_power_get_thermal() -> u32 {
    // Return CPU temperature in Celsius
    let power_service = services::power::POWER_SERVICE.lock();
    power_service.get_thermal_status().cpu_temp as u32
}



