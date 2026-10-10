// Kernel logging subsystem (dmesg ring buffer) for NebulaOS.
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

#pragma once

#include <cstdint>
#include <cstddef>

namespace kernel::log {

// Log levels, matching syslog severity.
enum class Level : std::uint8_t {
    EMERG   = 0,  // System is unusable
    ALERT   = 1,  // Action must be taken immediately
    CRIT    = 2,  // Critical conditions
    ERR     = 3,  // Error conditions
    WARNING = 4,  // Warning conditions
    NOTICE  = 5,  // Normal but significant conditions
    INFO    = 6,  // Informational messages
    DEBUG   = 7,  // Debug-level messages
};

// Initialize the logging subsystem.
void initialize();

// Set the minimum level that is recorded and echoed to serial.
void set_level(Level level);

// Get the current minimum level.
Level current_level() noexcept;

// Write a single character to the log (used by low-level putc).
void put(char c);

// Write a null-terminated string to the log.
void write(const char* text);

// Write a formatted line with a level prefix. The caller is responsible
// for ensuring the buffer is large enough; this is the primitive the
// higher level macros expand to.
void write_line(Level level, const char* text);

// Convenience helpers that pick the level for the caller.
inline void emerg(const char* text)   { write_line(Level::EMERG, text); }
inline void alert(const char* text)   { write_line(Level::ALERT, text); }
inline void crit(const char* text)    { write_line(Level::CRIT, text); }
inline void err(const char* text)     { write_line(Level::ERR, text); }
inline void warning(const char* text) { write_line(Level::WARNING, text); }
inline void notice(const char* text)  { write_line(Level::NOTICE, text); }
inline void info(const char* text)    { write_line(Level::INFO, text); }
inline void debug(const char* text)   { write_line(Level::DEBUG, text); }

// ---- Ring buffer access (for /proc/klog) ----

// Number of bytes currently stored in the ring buffer.
std::size_t available() noexcept;

// Read up to `size` bytes from the ring buffer into `buffer`, advancing
// the read cursor. Returns the number of bytes actually read.
std::size_t read(void* buffer, std::size_t size) noexcept;

// Peek at the most recent entry without consuming it.
std::size_t peek_last(void* buffer, std::size_t size) noexcept;

// Drop all buffered log entries.
void clear() noexcept;

// Statistics about the ring buffer.
struct LogStats {
    std::size_t total_written;   // Total bytes ever written (may wrap)
    std::size_t dropped;         // Bytes dropped due to overflow
    std::size_t buffer_size;     // Total capacity of the ring buffer
    std::size_t available_now;   // Bytes currently readable
};

LogStats stats() noexcept;

// ---- Syslog-style structured access ----

// Read log entries with level filtering. Returns number of entries read.
// Each entry contains: level, timestamp (seconds, nanoseconds), message.
// If buffer is null, returns total available entries matching filter.
int syslog_read(void* buffer, int count, int level_filter) noexcept;

// Clear the kernel log ring buffer.
void syslog_clear() noexcept;

// Get log statistics.
LogStats syslog_stats() noexcept;

} // namespace kernel::log