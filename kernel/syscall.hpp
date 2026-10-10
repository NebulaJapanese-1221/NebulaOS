// Syscall interface for the NebulaOS x86 operating system.
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or (at your
// option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// See LICENCE for the full license text.

#pragma once

namespace kernel::syscall {

void initialize();

// Registers a new file descriptor backed by a userspace buffer. Used by the
// userspace runtime to expose the framebuffer and the serial port without
// requiring a real filesystem in the kernel.
int open_device(const char* name, unsigned int buffer, unsigned int length);
void close_device(int fd);

// Fills the userspace struct with the current framebuffer geometry so a
// graphical userspace program can draw straight into the shared buffer.
bool get_framebuffer_info(void* info);

// Returns the number of whole seconds since boot and the tick count.
unsigned long long get_time_seconds();
unsigned int get_ticks();

// Yields the calling thread for the given number of ticks.
void sleep_ticks(unsigned int ticks);

// Kernel logging (syslog / dmesg)
struct LogEntry {
    unsigned char level;
    unsigned int timestamp_sec;
    unsigned int timestamp_nsec;
    char message[256];
};

// Read kernel log entries. Returns number of entries read.
// If buffer is null, returns total available entries.
int syslog_read(LogEntry* buffer, int count, int level_filter);

// Clear the kernel log ring buffer.
void syslog_clear();

// Get log statistics.
struct LogStats {
    unsigned int total_written;
    unsigned int dropped;
    unsigned int buffer_size;
    unsigned int available_now;
};
void syslog_stats(LogStats* stats);

// ---- Window manager syscalls ----

// Window lifecycle
int wm_create_window(const char* title, int x, int y, int w, int h);
void wm_destroy_window(int window_id);
void wm_show_window(int window_id);
void wm_hide_window(int window_id);

// Window state
void wm_set_title(int window_id, const char* title);
void wm_move_window(int window_id, int x, int y);
void wm_resize_window(int window_id, int w, int h);
void wm_minimize_window(int window_id);
void wm_maximize_window(int window_id);
void wm_restore_window(int window_id);

// Window drawing
void wm_clear_window(int window_id, unsigned int color);
void wm_fill_rect(int window_id, int x, int y, int w, int h, unsigned int color);
void wm_draw_text(int window_id, int x, int y, const char* text, unsigned int color, unsigned int scale);
void wm_present_window(int window_id);

// Window buffer access
unsigned int wm_get_buffer(int window_id);
unsigned int wm_get_buffer_width(int window_id);
unsigned int wm_get_buffer_height(int window_id);

// Event handling
int wm_poll_event(int window_id, void* event);
int wm_wait_event(int window_id, void* event, unsigned int timeout_ticks);

// Window info
bool wm_get_window_info(int window_id, void* info);

// Desktop / screen info
void wm_get_screen_info(void* info);

// Mouse / keyboard input for focused window
bool wm_poll_mouse(void* state);
bool wm_poll_keyboard(void* event);

// System info syscall
struct SysInfo {
    // Memory
    unsigned int total_memory_mb;
    unsigned int free_memory_mb;
    unsigned int used_memory_mb;
    
    // Uptime
    unsigned int uptime_seconds;
    
    // CPU info
    unsigned int cpu_count;
    unsigned int cpu_frequency_mhz;
    char cpu_vendor[13];  // 12 chars + null
    char cpu_model[49];   // 48 chars + null
    
    // Kernel info
    char kernel_version[64];
    char build_date[32];
    char build_sha[32];
};

// Get system information
void sysinfo(SysInfo* info);

} // namespace kernel::syscall