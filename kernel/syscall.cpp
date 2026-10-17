// Syscall dispatch for the NebulaOS x86 operating system.
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// See LICENCE for the full license text.

#include "syscall.hpp"
#include "../drivers/serial.hpp"
#include "../drivers/speaker.hpp"
#include "framebuffer.hpp"
#include "timer.hpp"
#include "log.hpp"
#include "wm.hpp"
#include "process.hpp"
#include "pmm.hpp"
#include "heap.hpp"
#include <cstring>

extern "C" {
extern const char git_build_date[];
extern const char git_build_sha[];
}

namespace kernel::syscall {

namespace {
struct RegisterState {
    unsigned int edi, esi, ebp, esp, ebx, edx, ecx, eax;
    unsigned int vector, error_code;
    unsigned int eip, cs, eflags, user_esp, user_ss;
};

using SyscallHandler = unsigned int (*)(RegisterState*);

constexpr unsigned int MAX_SYSCALL_NUMBER = 305;
SyscallHandler handlers[MAX_SYSCALL_NUMBER + 1] = {};

unsigned int sys_write(RegisterState* regs) {
    unsigned int fd = regs->ebx;
    const char* buf = reinterpret_cast<const char*>(regs->ecx);
    unsigned int count = regs->edx;

    if (fd == 1 || fd == 2) {
        for (unsigned int i = 0; i < count; ++i) {
            drivers::serial::write(buf[i]);
        }
        return count;
    }
    return 0xFFFFFFFF;
}

unsigned int sys_exit(RegisterState* regs) {
    (void)regs;
    for (;;) {
        asm volatile("cli; hlt");
    }
    return 0;
}

unsigned int sys_getpid(RegisterState* regs) {
    (void)regs;
    return 1;
}

unsigned int sys_open_device(RegisterState* regs) {
    const char* name = reinterpret_cast<const char*>(regs->ecx);

    // fd 3 is the framebuffer, fd 4 is the serial console.
    if (name == nullptr) {
        return 0xFFFFFFFF;
    }
    if (name[0] == 'f' && name[1] == 'b' && name[2] == '\0') {
        return 3;
    }
    if (name[0] == 's' && name[1] == 'e' && name[2] == 'r' && name[3] == '\0') {
        return 4;
    }
    return 0xFFFFFFFF;
}

unsigned int sys_get_framebuffer_info(RegisterState* regs) {
    struct __attribute__((packed)) Info {
        unsigned int buffer;
        unsigned int width;
        unsigned int height;
        unsigned int pitch;
        unsigned char bpp;
        unsigned char type;
        unsigned char red_position;
        unsigned char red_mask_size;
        unsigned char green_position;
        unsigned char green_mask_size;
        unsigned char blue_position;
        unsigned char blue_mask_size;
    };

    Info* info = reinterpret_cast<Info*>(regs->ecx);
    if (info == nullptr) {
        return 0;
    }
    info->buffer = reinterpret_cast<unsigned int>(kernel::framebuffer::buffer());
    info->width = kernel::framebuffer::width();
    info->height = kernel::framebuffer::height();
    info->pitch = 0;
    info->bpp = 32;
    info->type = 0;
    info->red_position = 16;
    info->red_mask_size = 8;
    info->green_position = 8;
    info->green_mask_size = 8;
    info->blue_position = 0;
    info->blue_mask_size = 8;
    return 1;
}

unsigned int sys_get_time(RegisterState* regs) {
    (void)regs;
    return static_cast<unsigned int>(kernel::timer::seconds());
}

unsigned int sys_get_ticks(RegisterState* regs) {
    (void)regs;
    return kernel::timer::ticks();
}

// Syslog syscalls
unsigned int sys_syslog_read(RegisterState* regs) {
    // EBX = buffer pointer (LogEntry*)
    // ECX = count
    // EDX = level filter (optional, -1 for all)
    LogEntry* buffer = reinterpret_cast<LogEntry*>(regs->ebx);
    int count = static_cast<int>(regs->ecx);
    int level_filter = static_cast<int>(regs->edx);
    
    if (buffer == nullptr) {
        // Return total available entries
        return static_cast<unsigned int>(kernel::log::syslog_read(nullptr, count, level_filter));
    }
    
    return static_cast<unsigned int>(kernel::log::syslog_read(buffer, count, level_filter));
}

unsigned int sys_syslog_clear(RegisterState* regs) {
    (void)regs;
    kernel::log::syslog_clear();
    return 0;
}

unsigned int sys_syslog_stats(RegisterState* regs) {
    // EBX = LogStats*
    LogStats* stats = reinterpret_cast<LogStats*>(regs->ebx);
    if (stats == nullptr) {
        return 0xFFFFFFFF;
    }
    const kernel::log::LogStats s = kernel::log::syslog_stats();
    stats->total_written = static_cast<unsigned int>(s.total_written);
    stats->dropped = static_cast<unsigned int>(s.dropped);
    stats->buffer_size = static_cast<unsigned int>(s.buffer_size);
    stats->available_now = static_cast<unsigned int>(s.available_now);
    return 0;
}

// Clipboard syscalls
unsigned int sys_clipboard_set(RegisterState* regs) {

    const void* data = reinterpret_cast<const void*>(regs->ebx);
    unsigned int size = regs->ecx;
    kernel::wm::wm_clipboard_set(data, size);
    return 0;
}

unsigned int sys_clipboard_get(RegisterState* regs) {
    void* buffer = reinterpret_cast<void*>(regs->ebx);
    unsigned int size = regs->ecx;
    return kernel::wm::wm_clipboard_get(buffer, size);
}

unsigned int sys_clipboard_size(RegisterState* regs) {
    (void)regs;
    return kernel::wm::wm_clipboard_size();
}

// PC speaker beep. Programs PIT channel 2 for the given frequency and gates
// the speaker for the given duration.
unsigned int sys_beep(RegisterState* regs) {
    unsigned int frequency = regs->ebx;
    unsigned int duration_ms = regs->ecx;
    drivers::speaker::beep(frequency, duration_ms);
    return 0;
}

// Sysinfo
unsigned int sys_sysinfo(RegisterState* regs) {
    SysInfo* info = reinterpret_cast<SysInfo*>(regs->ebx);
    if (info == nullptr) {
        return 0xFFFFFFFF;
    }

    // Memory
    unsigned long long detected = kernel::memory::pmm::total_bytes();
    info->total_memory_mb = static_cast<unsigned int>(detected / (1024ULL * 1024ULL));
    info->free_memory_mb = static_cast<unsigned int>(
        (kernel::memory::pmm::free_frames() * kernel::memory::pmm::PAGE_SIZE) /
        (1024ULL * 1024ULL));
    info->used_memory_mb = info->total_memory_mb > info->free_memory_mb
                               ? info->total_memory_mb - info->free_memory_mb
                               : 0;

    // Uptime
    info->uptime_seconds = kernel::timer::seconds();

    // CPU
    info->cpu_count = 1;
    info->cpu_frequency_mhz = 0;
    std::memset(info->cpu_vendor, 0, sizeof(info->cpu_vendor));
    std::memset(info->cpu_model, 0, sizeof(info->cpu_model));

    // Kernel info
    std::strncpy(info->kernel_version, "NebulaOS 0.0.1 Alpha",
                 sizeof(info->kernel_version) - 1);
    info->kernel_version[sizeof(info->kernel_version) - 1] = '\0';
    std::strncpy(info->build_date, ::git_build_date, sizeof(info->build_date) - 1);
    info->build_date[sizeof(info->build_date) - 1] = '\0';
    std::strncpy(info->build_sha, ::git_build_sha, sizeof(info->build_sha) - 1);
    info->build_sha[sizeof(info->build_sha) - 1] = '\0';

    return 0;
}

// Process list snapshot
unsigned int sys_process_list(RegisterState* regs) {
    ProcessInfo* out = reinterpret_cast<ProcessInfo*>(regs->ebx);
    unsigned int max = regs->ecx;
    if (out == nullptr || max == 0) {
        return 0;
    }

    kernel::process::Process* processes[64];
    std::size_t count = kernel::process::list(processes, 64);
    if (count > max) {
        count = max;
    }

    for (std::size_t i = 0; i < count; ++i) {
        out[i].id = processes[i]->id;
        out[i].state = static_cast<unsigned int>(processes[i]->state);
        out[i].priority = 128;
        out[i].total_time_ms = 0;
        std::strncpy(out[i].name, kernel::process::name(processes[i]),
                     sizeof(out[i].name) - 1);
        out[i].name[sizeof(out[i].name) - 1] = '\0';
    }

    return static_cast<unsigned int>(count);
}

} // namespace

void initialize() {
    for (unsigned int i = 0; i <= MAX_SYSCALL_NUMBER; ++i) {
        handlers[i] = nullptr;
    }
    handlers[1] = sys_write;   // write
    handlers[4] = sys_write;   // write (Linux compat)
    handlers[60] = sys_exit;   // exit
    handlers[20] = sys_getpid; // getpid
    handlers[200] = sys_open_device;          // open_device
    handlers[201] = sys_get_framebuffer_info; // get_framebuffer_info
    handlers[202] = sys_get_time;             // get_time_seconds
    handlers[203] = sys_get_ticks;            // get_ticks
    handlers[210] = sys_syslog_read;          // syslog_read
    handlers[211] = sys_syslog_clear;         // syslog_clear
    handlers[212] = sys_syslog_stats;         // syslog_stats
    handlers[300] = sys_clipboard_set;        // clipboard_set
    handlers[301] = sys_clipboard_get;        // clipboard_get
    handlers[302] = sys_clipboard_size;       // clipboard_size
    handlers[303] = sys_beep;                 // beep
    handlers[304] = sys_sysinfo;              // sysinfo
    handlers[305] = sys_process_list;         // process_list
}

extern "C" unsigned int syscall_dispatch(void* register_state) {
    RegisterState* regs = static_cast<RegisterState*>(register_state);
    unsigned int syscall_num = regs->eax;

    if (syscall_num <= MAX_SYSCALL_NUMBER && handlers[syscall_num] != nullptr) {
        return handlers[syscall_num](regs);
    }

    drivers::serial::write("[syscall] unknown syscall: ");
    drivers::serial::write_decimal(syscall_num);
    drivers::serial::write_newline();

    return 0xFFFFFFFF;
}

} // namespace kernel::syscall