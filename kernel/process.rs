use crate::drivers::paging;
use crate::{memory::frame_allocator, tss};

const MAX_PROCESSES: usize = 8;
const STACK_FRAME_WORDS: usize = 19;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProcessState {
    Unused,
    Ready,
    Running,
    Zombie,
}

#[derive(Clone, Copy)]
struct Process {
    saved_esp: u32,
    page_directory: u32,
    kernel_stack_frame: usize,
    kernel_stack_top: u32,
    state: ProcessState,
}

impl Process {
    const EMPTY: Self = Self {
        saved_esp: 0,
        page_directory: 0,
        kernel_stack_frame: 0,
        kernel_stack_top: 0,
        state: ProcessState::Unused,
    };
}

static mut PROCESSES: [Process; MAX_PROCESSES] = [Process::EMPTY; MAX_PROCESSES];
static mut CURRENT_PID: usize = 0;
static mut KERNEL_IDLE_ESP: u32 = 0;
static mut KERNEL_STACK_TOP: u32 = 0;

#[no_mangle]
static mut SCHEDULED_CR3: u32 = 0;

pub fn init() {
    unsafe {
        CURRENT_PID = 0;
        KERNEL_IDLE_ESP = 0;
        KERNEL_STACK_TOP = tss::tss_esp0();
        SCHEDULED_CR3 = paging::kernel_page_dir();
    }
}

pub fn add_user_process(
    entry: u32,
    user_stack: u32,
    page_directory: u32,
    kernel_stack_frame: usize,
) -> Option<u32> {
    unsafe {
        let flags: u32;
        core::arch::asm!(
            "pushfd",
            "pop {0:e}",
            "cli",
            out(reg) flags,
            options(preserves_flags)
        );
        let processes = core::ptr::addr_of_mut!(PROCESSES).cast::<Process>();
        let mut available_slot = None;
        for index in 0..MAX_PROCESSES {
            if (*processes.add(index)).state == ProcessState::Unused {
                available_slot = Some(index);
                break;
            }
        }
        let slot = match available_slot {
            Some(slot) => slot,
            None => {
                core::arch::asm!(
                    "push {0:e}",
                    "popfd",
                    in(reg) flags,
                    options(preserves_flags)
                );
                return None;
            }
        };
        let kernel_stack_top = (kernel_stack_frame + 4096) as u32;
        let saved_esp = kernel_stack_top - (STACK_FRAME_WORDS * core::mem::size_of::<u32>()) as u32;
        let frame = saved_esp as *mut u32;
        let user_data = crate::gdt::user_data_selector() as u32;

        *frame.add(0) = user_data;
        *frame.add(1) = user_data;
        *frame.add(2) = user_data;
        *frame.add(3) = user_data;
        for index in 4..12 {
            *frame.add(index) = 0;
        }
        *frame.add(12) = 32;
        *frame.add(13) = 0;
        *frame.add(14) = entry;
        *frame.add(15) = crate::gdt::user_code_selector() as u32;
        *frame.add(16) = 0x202;
        *frame.add(17) = user_stack;
        *frame.add(18) = user_data;

        *processes.add(slot) = Process {
            saved_esp,
            page_directory,
            kernel_stack_frame,
            kernel_stack_top,
            state: ProcessState::Ready,
        };
        core::arch::asm!("push {0:e}", "popfd", in(reg) flags, options(preserves_flags));
        Some((slot + 1) as u32)
    }
}

pub fn terminate_current_user_process() -> bool {
    unsafe {
        if CURRENT_PID == 0 {
            return false;
        }
        let process = &mut PROCESSES[CURRENT_PID - 1];
        if process.state != ProcessState::Running {
            return false;
        }
        process.state = ProcessState::Zombie;
        true
    }
}

#[no_mangle]
pub extern "C" fn schedule_rust(current_esp: u32) -> u32 {
    unsafe {
        if CURRENT_PID == 0 {
            KERNEL_IDLE_ESP = current_esp;
        } else {
            let current = &mut PROCESSES[CURRENT_PID - 1];
            if current.state == ProcessState::Running {
                current.saved_esp = current_esp;
                current.state = ProcessState::Ready;
            }
        }

        reap_zombies();

        let start = CURRENT_PID;
        for offset in 1..=MAX_PROCESSES {
            let slot = (start + offset - 1) % MAX_PROCESSES;
            if PROCESSES[slot].state == ProcessState::Ready {
                let process = &mut PROCESSES[slot];
                process.state = ProcessState::Running;
                CURRENT_PID = slot + 1;
                SCHEDULED_CR3 = process.page_directory;
                tss::set_kernel_stack(process.kernel_stack_top);
                return process.saved_esp;
            }
        }

        CURRENT_PID = 0;
        SCHEDULED_CR3 = paging::kernel_page_dir();
        tss::set_kernel_stack(KERNEL_STACK_TOP);
        KERNEL_IDLE_ESP
    }
}

unsafe fn reap_zombies() {
    let processes = core::ptr::addr_of_mut!(PROCESSES).cast::<Process>();
    for index in 0..MAX_PROCESSES {
        if index + 1 == CURRENT_PID {
            continue;
        }
        let process = &mut *processes.add(index);
        if process.state != ProcessState::Zombie {
            continue;
        }
        paging::destroy_user_address_space(process.page_directory);
        frame_allocator::free_frame(process.kernel_stack_frame);
        *process = Process::EMPTY;
    }
}
