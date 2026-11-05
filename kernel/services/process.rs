// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

use crate::arch::{gdt, smp, tss};
use crate::memory::{frame_allocator, paging};
use crate::sync::SpinLock;
use crate::userspace::syscall::{ProcessEntry, ProcessInfo};

const MAX_PROCESSES: usize = 8;
const STACK_FRAME_WORDS: usize = 19;
const EVENT_QUEUE_SIZE: usize = 16;
const REG_INTERRUPT: usize = 12;
const NO_CPU: usize = usize::MAX;

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
    running_cpu: usize,
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
        running_cpu: NO_CPU,
        state: ProcessState::Unused,
    };
}

#[derive(Clone, Copy)]
struct CpuScheduler {
    current_pid: usize,
    idle_esp: u32,
    kernel_stack_top: u32,
    pending_process: usize,
}

impl CpuScheduler {
    const EMPTY: Self = Self {
        current_pid: 0,
        idle_esp: 0,
        kernel_stack_top: 0,
        pending_process: NO_CPU,
    };
}

static PROCESSES: SpinLock<[Process; MAX_PROCESSES]> =
    SpinLock::new([Process::EMPTY; MAX_PROCESSES]);
static mut CPU_SCHEDULERS: [CpuScheduler; smp::MAX_CPUS] = [CpuScheduler::EMPTY; smp::MAX_CPUS];

pub fn init() {
    *PROCESSES.lock() = [Process::EMPTY; MAX_PROCESSES];
    unsafe {
        core::ptr::addr_of_mut!(CPU_SCHEDULERS).write([CpuScheduler::EMPTY; smp::MAX_CPUS]);
    }
    init_cpu(0, tss::tss_esp0());
}

pub fn init_cpu(cpu_index: usize, kernel_stack_top: u32) {
    if cpu_index >= smp::MAX_CPUS {
        return;
    }
    unsafe {
        let cpu = &mut *core::ptr::addr_of_mut!(CPU_SCHEDULERS)
            .cast::<CpuScheduler>()
            .add(cpu_index);
        *cpu = CpuScheduler {
            kernel_stack_top,
            ..CpuScheduler::EMPTY
        };
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
    let kernel_stack_top = (kernel_stack_frame + 4096) as u32;
    let saved_esp = kernel_stack_top - (STACK_FRAME_WORDS * core::mem::size_of::<u32>()) as u32;
    let frame = saved_esp as *mut u32;
    let user_data = gdt::user_data_selector() as u32;
    unsafe {
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
    }

    let mut processes = PROCESSES.lock();
    let slot = processes
        .iter()
        .position(|process| process.state == ProcessState::Unused)?;
    processes[slot] = Process {
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
        running_cpu: NO_CPU,
        state: ProcessState::Ready,
    };
    Some((slot + 1) as u32)
}

pub fn current_app_context() -> Option<(u32, AppRect)> {
    let cpu_index = smp::current_cpu_index();
    let current_pid = unsafe { CPU_SCHEDULERS[cpu_index].current_pid };
    let processes = PROCESSES.lock();
    if current_pid == 0 {
        return None;
    }
    let process = &processes[current_pid - 1];
    (process.state == ProcessState::Running).then_some((process.app_id, process.content))
}

pub fn current_pid() -> u32 {
    let cpu_index = smp::current_cpu_index();
    unsafe { CPU_SCHEDULERS[cpu_index].current_pid as u32 }
}

pub fn take_current_event() -> Option<UserEvent> {
    let current_pid = current_pid() as usize;
    if current_pid == 0 {
        return None;
    }
    let mut processes = PROCESSES.lock();
    let process = &mut processes[current_pid - 1];
    if process.state != ProcessState::Running || process.event_count == 0 {
        return None;
    }
    let event = process.events[process.event_head];
    process.event_head = (process.event_head + 1) % EVENT_QUEUE_SIZE;
    process.event_count -= 1;
    Some(event)
}

pub fn block_current_user_process() -> bool {
    let current_pid = current_pid() as usize;
    if current_pid == 0 {
        return false;
    }
    let mut processes = PROCESSES.lock();
    let process = &mut processes[current_pid - 1];
    if process.state != ProcessState::Running {
        return false;
    }
    process.sleep_active = false;
    process.state = ProcessState::Blocked;
    true
}

pub fn yield_current_user_process() -> bool {
    let current_pid = current_pid() as usize;
    if current_pid == 0 {
        return false;
    }
    let mut processes = PROCESSES.lock();
    let process = &mut processes[current_pid - 1];
    if process.state != ProcessState::Running {
        return false;
    }
    process.state = ProcessState::Ready;
    true
}

pub fn sleep_current_user_process(now: u32, ticks: u32) -> bool {
    let current_pid = current_pid() as usize;
    if current_pid == 0 {
        return false;
    }
    let mut processes = PROCESSES.lock();
    let process = &mut processes[current_pid - 1];
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

pub fn wake_sleeping_processes(now: u32) {
    let mut processes = PROCESSES.lock();
    for process in processes.iter_mut() {
        if process.state == ProcessState::Blocked
            && process.sleep_active
            && now.wrapping_sub(process.wake_tick) < 0x8000_0000
        {
            process.sleep_active = false;
            process.state = ProcessState::Ready;
        }
    }
}

pub fn process_info() -> ProcessInfo {
    let current_pid = current_pid();
    let processes = PROCESSES.lock();
    let mut info = ProcessInfo {
        current_pid,
        ..ProcessInfo::default()
    };
    for (index, process) in processes.iter().enumerate() {
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

pub fn take_current_draw_dirty() -> Option<AppRect> {
    let current_pid = current_pid() as usize;
    if current_pid == 0 {
        return None;
    }
    let mut processes = PROCESSES.lock();
    let process = &mut processes[current_pid - 1];
    let dirty = process.state == ProcessState::Running && process.draw_dirty;
    process.draw_dirty = false;
    dirty.then_some(process.content)
}

pub fn mark_current_draw_dirty() {
    let current_pid = current_pid() as usize;
    if current_pid == 0 {
        return;
    }
    let mut processes = PROCESSES.lock();
    if processes[current_pid - 1].state == ProcessState::Running {
        processes[current_pid - 1].draw_dirty = true;
    }
}

pub fn route_event(pid: u32, event: UserEvent) -> bool {
    let mut processes = PROCESSES.lock();
    enqueue_event(&mut processes, pid, event)
}

pub fn update_content_rect(
    pid: u32,
    content: AppRect,
    pointer_x: u32,
    pointer_y: u32,
    left_down: bool,
) -> bool {
    let Some(index) = process_index(pid) else {
        return false;
    };
    let mut processes = PROCESSES.lock();
    let Some(process) = processes.get_mut(index) else {
        return false;
    };
    if process.state == ProcessState::Unused || process.state == ProcessState::Zombie {
        return false;
    }
    process.content = content;
    enqueue_event(
        &mut processes,
        pid,
        UserEvent {
            kind: 3,
            x: pointer_x,
            y: pointer_y,
            left_down: u32::from(left_down),
            ..UserEvent::default()
        },
    )
}

pub fn process_exists(pid: u32) -> bool {
    let Some(index) = process_index(pid) else {
        return false;
    };
    let processes = PROCESSES.lock();
    processes.get(index).is_some_and(|process| {
        process.state != ProcessState::Unused && process.state != ProcessState::Zombie
    })
}

pub fn terminate_process(pid: u32) -> bool {
    let Some(index) = process_index(pid) else {
        return false;
    };
    let mut processes = PROCESSES.lock();
    let Some(process) = processes.get_mut(index) else {
        return false;
    };
    if process.state == ProcessState::Unused || process.state == ProcessState::Zombie {
        return false;
    }
    process.state = ProcessState::Zombie;
    true
}

pub fn terminate_current_user_process() -> bool {
    let pid = current_pid();
    pid != 0 && terminate_process(pid)
}

fn process_index(pid: u32) -> Option<usize> {
    pid.checked_sub(1)
        .map(|index| index as usize)
        .filter(|index| *index < MAX_PROCESSES)
}

fn enqueue_event(processes: &mut [Process; MAX_PROCESSES], pid: u32, event: UserEvent) -> bool {
    let Some(index) = process_index(pid) else {
        return false;
    };
    let process = &mut processes[index];
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

#[no_mangle]
pub extern "C" fn schedule_rust(current_esp: u32) -> u32 {
    let cpu_index = smp::current_cpu_index().min(smp::MAX_CPUS - 1);
    let interrupt = unsafe { *((current_esp as *const u32).add(REG_INTERRUPT)) };
    let mut processes = PROCESSES.lock();

    unsafe {
        let cpu = &mut *core::ptr::addr_of_mut!(CPU_SCHEDULERS)
            .cast::<CpuScheduler>()
            .add(cpu_index);

        if interrupt == 128
            && cpu.current_pid != 0
            && processes[cpu.current_pid - 1].state == ProcessState::Running
        {
            return current_esp;
        }

        if cpu.pending_process != NO_CPU {
            let previous = &mut processes[cpu.pending_process];
            if previous.running_cpu == cpu_index {
                previous.running_cpu = NO_CPU;
            }
            cpu.pending_process = NO_CPU;
        }

        let old_pid = cpu.current_pid;
        if old_pid == 0 {
            cpu.idle_esp = current_esp;
        } else {
            let current = &mut processes[old_pid - 1];
            if current.state != ProcessState::Unused {
                current.saved_esp = current_esp;
                if current.state == ProcessState::Running {
                    current.state = ProcessState::Ready;
                }
            }
        }

        reap_zombies(&mut processes);

        for offset in 1..=MAX_PROCESSES {
            let slot = (old_pid + offset - 1) % MAX_PROCESSES;
            if processes[slot].state == ProcessState::Ready && processes[slot].running_cpu == NO_CPU
            {
                let process = &mut processes[slot];
                process.state = ProcessState::Running;
                process.running_cpu = cpu_index;
                cpu.current_pid = slot + 1;
                tss::set_kernel_stack(cpu_index, process.kernel_stack_top);
                paging::activate_page_dir(process.page_directory);
                if old_pid != 0 && slot + 1 != old_pid {
                    cpu.pending_process = old_pid - 1;
                }
                return process.saved_esp;
            }
        }

        if old_pid != 0 {
            let current = &mut processes[old_pid - 1];
            if current.state == ProcessState::Ready && current.running_cpu == cpu_index {
                current.state = ProcessState::Running;
                tss::set_kernel_stack(cpu_index, current.kernel_stack_top);
                paging::activate_page_dir(current.page_directory);
                return current_esp;
            }
            if current.running_cpu == cpu_index {
                cpu.pending_process = old_pid - 1;
            }
        }

        cpu.current_pid = 0;
        tss::set_kernel_stack(cpu_index, cpu.kernel_stack_top);
        paging::activate_page_dir(paging::kernel_page_dir());
        cpu.idle_esp
    }
}

fn reap_zombies(processes: &mut [Process; MAX_PROCESSES]) {
    for process in processes {
        if process.state != ProcessState::Zombie || process.running_cpu != NO_CPU {
            continue;
        }
        paging::destroy_user_address_space(process.page_directory);
        frame_allocator::free_frame(process.kernel_stack_frame);
        *process = Process::EMPTY;
    }
}
