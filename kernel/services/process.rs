use crate::arch::{gdt, tss};
use crate::memory::{frame_allocator, paging};
use crate::userspace::syscall::{ProcessEntry, ProcessInfo};

const MAX_PROCESSES: usize = 8;
const STACK_FRAME_WORDS: usize = 19;
const EVENT_QUEUE_SIZE: usize = 16;
const REG_INTERRUPT: usize = 12;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct UserEvent {
    pub kind: u32,
    pub key: u32,
    pub x: u32,
    pub y: u32,
    pub left_down: u32,
}

#[derive(Clone, Copy, Default)]
pub struct AppRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProcessState {
    Unused,
    Ready,
    Running,
    Blocked,
    Zombie,
}

#[derive(Clone, Copy)]
struct Process {
    saved_esp: u32,
    page_directory: u32,
    kernel_stack_frame: usize,
    kernel_stack_top: u32,
    app_id: u32,
    content: AppRect,
    events: [UserEvent; EVENT_QUEUE_SIZE],
    event_head: usize,
    event_count: usize,
    draw_dirty: bool,
    wake_tick: u32,
    sleep_active: bool,
    state: ProcessState,
}

impl Process {
    const EMPTY: Self = Self {
        saved_esp: 0,
        page_directory: 0,
        kernel_stack_frame: 0,
        kernel_stack_top: 0,
        app_id: 0,
        content: AppRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
        events: [UserEvent {
            kind: 0,
            key: 0,
            x: 0,
            y: 0,
            left_down: 0,
        }; EVENT_QUEUE_SIZE],
        event_head: 0,
        event_count: 0,
        draw_dirty: false,
        wake_tick: 0,
        sleep_active: false,
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
    app_id: u32,
    content: AppRect,
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
        let user_data = gdt::user_data_selector() as u32;

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
        *frame.add(15) = gdt::user_code_selector() as u32;
        *frame.add(16) = 0x202;
        *frame.add(17) = user_stack;
        *frame.add(18) = user_data;

        *processes.add(slot) = Process {
            saved_esp,
            page_directory,
            kernel_stack_frame,
            kernel_stack_top,
            app_id,
            content,
            events: [UserEvent::default(); EVENT_QUEUE_SIZE],
            event_head: 0,
            event_count: 0,
            draw_dirty: false,
            wake_tick: 0,
            sleep_active: false,
            state: ProcessState::Ready,
        };
        core::arch::asm!("push {0:e}", "popfd", in(reg) flags, options(preserves_flags));
        Some((slot + 1) as u32)
    }
}

pub fn current_app_context() -> Option<(u32, AppRect)> {
    unsafe {
        if CURRENT_PID == 0 {
            None
        } else {
            let process = &PROCESSES[CURRENT_PID - 1];
            (process.state == ProcessState::Running).then_some((process.app_id, process.content))
        }
    }
}

pub fn current_pid() -> u32 {
    unsafe { CURRENT_PID as u32 }
}

pub fn take_current_event() -> Option<UserEvent> {
    unsafe {
        if CURRENT_PID == 0 {
            return None;
        }
        let process = &mut PROCESSES[CURRENT_PID - 1];
        if process.state != ProcessState::Running || process.event_count == 0 {
            return None;
        }
        let event = process.events[process.event_head];
        process.event_head = (process.event_head + 1) % EVENT_QUEUE_SIZE;
        process.event_count -= 1;
        Some(event)
    }
}

pub fn block_current_user_process() -> bool {
    unsafe {
        if CURRENT_PID == 0 {
            return false;
        }
        let process = &mut PROCESSES[CURRENT_PID - 1];
        if process.state != ProcessState::Running {
            return false;
        }
        process.sleep_active = false;
        process.state = ProcessState::Blocked;
        true
    }
}

pub fn yield_current_user_process() -> bool {
    unsafe {
        if CURRENT_PID == 0 {
            return false;
        }
        let process = &mut *core::ptr::addr_of_mut!(PROCESSES)
            .cast::<Process>()
            .add(CURRENT_PID - 1);
        if process.state != ProcessState::Running {
            return false;
        }
        process.state = ProcessState::Ready;
        true
    }
}

pub fn sleep_current_user_process(now: u32, ticks: u32) -> bool {
    unsafe {
        if CURRENT_PID == 0 {
            return false;
        }
        let process = &mut *core::ptr::addr_of_mut!(PROCESSES)
            .cast::<Process>()
            .add(CURRENT_PID - 1);
        if process.state != ProcessState::Running {
            return false;
        }
        if ticks == 0 {
            process.sleep_active = false;
            process.state = ProcessState::Ready;
        } else {
            process.wake_tick = now.wrapping_add(ticks);
            process.sleep_active = true;
            process.state = ProcessState::Blocked;
        }
        true
    }
}

pub fn wake_sleeping_processes(now: u32) {
    unsafe {
        let processes = core::ptr::addr_of_mut!(PROCESSES).cast::<Process>();
        for index in 0..MAX_PROCESSES {
            let process = &mut *processes.add(index);
            if process.state == ProcessState::Blocked
                && process.sleep_active
                && now.wrapping_sub(process.wake_tick) < 0x8000_0000
            {
                process.sleep_active = false;
                process.state = ProcessState::Ready;
            }
        }
    }
}

pub fn process_info() -> ProcessInfo {
    unsafe {
        let mut info = ProcessInfo {
            current_pid: CURRENT_PID as u32,
            ..ProcessInfo::default()
        };
        let processes = core::ptr::addr_of!(PROCESSES).cast::<Process>();
        for index in 0..MAX_PROCESSES {
            let process = &*processes.add(index);
            let state = match process.state {
                ProcessState::Unused | ProcessState::Zombie => continue,
                ProcessState::Ready => 1,
                ProcessState::Running => 2,
                ProcessState::Blocked => 3,
            };
            let Some(output) = info.entries.get_mut(info.count as usize) else {
                break;
            };
            *output = ProcessEntry {
                pid: (index + 1) as u32,
                app_id: process.app_id,
                state,
                pending_events: process.event_count as u32,
            };
            info.count += 1;
        }
        info
    }
}

pub fn take_current_draw_dirty() -> bool {
    unsafe {
        if CURRENT_PID == 0 {
            return false;
        }
        let process = &mut PROCESSES[CURRENT_PID - 1];
        let dirty = process.state == ProcessState::Running && process.draw_dirty;
        process.draw_dirty = false;
        dirty
    }
}

pub fn mark_current_draw_dirty() {
    unsafe {
        if CURRENT_PID != 0 && PROCESSES[CURRENT_PID - 1].state == ProcessState::Running {
            PROCESSES[CURRENT_PID - 1].draw_dirty = true;
        }
    }
}

pub fn route_event(pid: u32, event: UserEvent) -> bool {
    let flags = disable_interrupts();
    let queued = unsafe { enqueue_event(pid, event) };
    restore_interrupts(flags);
    queued
}

pub fn update_content_rect(
    pid: u32,
    content: AppRect,
    pointer_x: u32,
    pointer_y: u32,
    left_down: bool,
) -> bool {
    let flags = disable_interrupts();
    let updated = unsafe {
        let Some(index) = pid.checked_sub(1).map(|index| index as usize) else {
            restore_interrupts(flags);
            return false;
        };
        if index < MAX_PROCESSES {
            let process = &mut *core::ptr::addr_of_mut!(PROCESSES)
                .cast::<Process>()
                .add(index);
            if process.state != ProcessState::Unused && process.state != ProcessState::Zombie {
                process.content = content;
                true
            } else {
                false
            }
        } else {
            false
        }
    };
    let queued = updated
        && unsafe {
            enqueue_event(
                pid,
                UserEvent {
                    kind: 3,
                    x: pointer_x,
                    y: pointer_y,
                    left_down: u32::from(left_down),
                    ..UserEvent::default()
                },
            )
        };
    restore_interrupts(flags);
    queued
}

pub fn process_exists(pid: u32) -> bool {
    unsafe {
        let Some(index) = pid.checked_sub(1).map(|index| index as usize) else {
            return false;
        };
        index < MAX_PROCESSES && {
            let process = &*core::ptr::addr_of!(PROCESSES).cast::<Process>().add(index);
            process.state != ProcessState::Unused && process.state != ProcessState::Zombie
        }
    }
}

pub fn terminate_process(pid: u32) -> bool {
    let flags = disable_interrupts();
    let terminated = unsafe {
        let Some(index) = pid.checked_sub(1).map(|index| index as usize) else {
            restore_interrupts(flags);
            return false;
        };
        if index < MAX_PROCESSES {
            let process = &mut *core::ptr::addr_of_mut!(PROCESSES)
                .cast::<Process>()
                .add(index);
            if process.state != ProcessState::Unused && process.state != ProcessState::Zombie {
                process.state = ProcessState::Zombie;
                true
            } else {
                false
            }
        } else {
            false
        }
    };
    restore_interrupts(flags);
    terminated
}

pub fn terminate_current_user_process() -> bool {
    unsafe { CURRENT_PID != 0 && terminate_process(CURRENT_PID as u32) }
}

unsafe fn enqueue_event(pid: u32, event: UserEvent) -> bool {
    let Some(index) = pid.checked_sub(1).map(|index| index as usize) else {
        return false;
    };
    if index >= MAX_PROCESSES {
        return false;
    }
    let process = &mut *core::ptr::addr_of_mut!(PROCESSES)
        .cast::<Process>()
        .add(index);
    if process.state == ProcessState::Unused || process.state == ProcessState::Zombie {
        return false;
    }
    if process.event_count == EVENT_QUEUE_SIZE {
        process.event_head = (process.event_head + 1) % EVENT_QUEUE_SIZE;
        process.event_count -= 1;
    }
    let tail = (process.event_head + process.event_count) % EVENT_QUEUE_SIZE;
    process.events[tail] = event;
    process.event_count += 1;
    if process.state == ProcessState::Blocked {
        process.sleep_active = false;
        process.state = ProcessState::Ready;
    }
    true
}

fn disable_interrupts() -> u32 {
    let flags: u32;
    unsafe {
        core::arch::asm!(
            "pushfd",
            "pop {0:e}",
            "cli",
            out(reg) flags,
            options(preserves_flags)
        );
    }
    flags
}

fn restore_interrupts(flags: u32) {
    unsafe {
        core::arch::asm!("push {0:e}", "popfd", in(reg) flags, options(preserves_flags));
    }
}

#[no_mangle]
pub extern "C" fn schedule_rust(current_esp: u32) -> u32 {
    unsafe {
        let interrupt = *((current_esp as *const u32).add(REG_INTERRUPT));
        if interrupt == 128
            && CURRENT_PID != 0
            && PROCESSES[CURRENT_PID - 1].state == ProcessState::Running
        {
            return current_esp;
        }

        if CURRENT_PID == 0 {
            KERNEL_IDLE_ESP = current_esp;
        } else {
            let current = &mut PROCESSES[CURRENT_PID - 1];
            if current.state != ProcessState::Zombie && current.state != ProcessState::Unused {
                current.saved_esp = current_esp;
                if current.state == ProcessState::Running {
                    current.state = ProcessState::Ready;
                }
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
