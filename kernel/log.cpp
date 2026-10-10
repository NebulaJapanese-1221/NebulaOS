// Kernel logging subsystem implementation for NebulaOS.
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

#include "log.hpp"
#include "../drivers/serial.hpp"
#include "timer.hpp"
#include <cstring>

namespace kernel::log {

namespace {

// 64 KiB ring buffer - big enough to hold a boot's worth of messages.
constexpr std::size_t RING_SIZE = 64 * 1024;

alignas(16) unsigned char ring_buffer[RING_SIZE];
std::size_t ring_write = 0;  // Write cursor (bytes written mod RING_SIZE)
std::size_t ring_read  = 0;  // Read cursor
std::size_t ring_bytes = 0;  // Bytes currently buffered
std::size_t total_written = 0;
std::size_t dropped = 0;

Level current_level_value = Level::INFO;

// ANSI color codes for serial output.
const char* level_prefix[] = {
    "\033[31mEMG",  // EMERG - red
    "\033[31mALR",  // ALERT - red
    "\033[31mCRT",  // CRIT - red
    "\033[31mERR",  // ERR - red
    "\033[33mWRN",  // WARNING - yellow
    "\033[36mNTC",  // NOTICE - cyan
    "\033[32mINF",  // INFO - green
    "\033[37mDBG",  // DEBUG - white
};

// Structure for parsed log entry in ring buffer
struct LogEntryInternal {
    std::uint8_t level;
    std::uint32_t timestamp_sec;
    std::uint32_t timestamp_nsec;
    char message[256];
};

void ring_put(unsigned char byte) {
    ring_buffer[ring_write] = byte;
    ring_write = (ring_write + 1) % RING_SIZE;
    if (ring_bytes < RING_SIZE) {
        ++ring_bytes;
    } else {
        // Buffer full: advance the read cursor to drop the oldest byte.
        ring_read = (ring_read + 1) % RING_SIZE;
        ++dropped;
    }
    ++total_written;
}

void ring_write_bytes(const unsigned char* data, std::size_t count) {
    for (std::size_t i = 0; i < count; ++i) {
        ring_put(data[i]);
    }
}

// Write a structured log entry to the ring buffer
void ring_write_entry(Level level, const char* text) {
    LogEntryInternal entry;
    entry.level = static_cast<std::uint8_t>(level);
    
    // Get current timestamp
    entry.timestamp_sec = static_cast<std::uint32_t>(kernel::timer::seconds());
    entry.timestamp_nsec = static_cast<std::uint32_t>(kernel::timer::nanoseconds() % 1000000000ULL);
    
    if (text != nullptr) {
        std::strncpy(entry.message, text, sizeof(entry.message) - 1);
        entry.message[sizeof(entry.message) - 1] = '\0';
    } else {
        entry.message[0] = '\0';
    }
    
    ring_write_bytes(reinterpret_cast<const unsigned char*>(&entry), sizeof(entry));
}

} // namespace

void initialize() {
    ring_write = 0;
    ring_read = 0;
    ring_bytes = 0;
    total_written = 0;
    dropped = 0;
    current_level_value = Level::INFO;
}

void set_level(Level level) {
    current_level_value = level;
}

Level current_level() noexcept {
    return current_level_value;
}

void put(char c) {
    ring_put(static_cast<unsigned char>(c));
    // Also mirror to serial when available.
    drivers::serial::write(c);
}

void write(const char* text) {
    if (text == nullptr) {
        return;
    }
    while (*text != '\0') {
        put(*text);
        ++text;
    }
}

void write_line(Level level, const char* text) {
    // Only record/echo messages at or above the configured level.
    if (static_cast<std::uint8_t>(level) > static_cast<std::uint8_t>(current_level_value)) {
        return;
    }

    // Echo to serial with a colored level prefix.
    const char* prefix = level_prefix[static_cast<std::uint8_t>(level)];
    drivers::serial::write(prefix);
    drivers::serial::write("] ");
    if (text != nullptr) {
        drivers::serial::write(text);
    }
    drivers::serial::write_newline();

    // Also store in the ring buffer as structured entry
    ring_write_entry(level, text);
}

std::size_t available() noexcept {
    return ring_bytes;
}

std::size_t read(void* buffer, std::size_t size) noexcept {
    if (buffer == nullptr || size == 0 || ring_bytes == 0) {
        return 0;
    }

    unsigned char* out = static_cast<unsigned char*>(buffer);
    std::size_t count = 0;
    while (count < size && ring_bytes > 0) {
        out[count] = ring_buffer[ring_read];
        ring_read = (ring_read + 1) % RING_SIZE;
        --ring_bytes;
        ++count;
    }
    return count;
}

std::size_t peek_last(void* buffer, std::size_t size) noexcept {
    if (buffer == nullptr || size == 0 || ring_bytes == 0) {
        return 0;
    }

    // Find the start of the last line by walking backwards from the
    // write cursor until we hit a newline or run out of buffered bytes.
    unsigned char* out = static_cast<unsigned char*>(buffer);
    std::size_t read_cursor = ring_write;
    if (read_cursor == 0) {
        read_cursor = RING_SIZE;
    }
    --read_cursor;  // Point at the last byte written.

    // Walk backwards to find the previous newline.
    std::size_t scan = ring_bytes;
    std::size_t line_start = read_cursor;
    while (scan > 0) {
        if (ring_buffer[read_cursor] == '\n') {
            line_start = (read_cursor + 1) % RING_SIZE;
            break;
        }
        read_cursor = (read_cursor == 0) ? RING_SIZE - 1 : read_cursor - 1;
        --scan;
    }

    // Now copy from line_start forward until newline or buffer exhausted.
    std::size_t count = 0;
    std::size_t cursor = line_start;
    scan = ring_bytes;
    while (count < size && scan > 0) {
        unsigned char c = ring_buffer[cursor];
        if (c == '\n') {
            break;
        }
        out[count++] = c;
        cursor = (cursor + 1) % RING_SIZE;
        --scan;
    }
    return count;
}

void clear() noexcept {
    ring_write = 0;
    ring_read = 0;
    ring_bytes = 0;
}

LogStats stats() noexcept {
    return {total_written, dropped, RING_SIZE, ring_bytes};
}

// Syslog-style functions

struct SyslogEntry {
    std::uint8_t level;
    std::uint32_t timestamp_sec;
    std::uint32_t timestamp_nsec;
    char message[256];
};

int syslog_read(void* buffer, int count, int level_filter) noexcept {
    if (count <= 0) {
        return 0;
    }
    
    // If buffer is null, return count of available entries matching filter
    bool just_count = (buffer == nullptr);
    
    SyslogEntry* entries = static_cast<SyslogEntry*>(buffer);
    int entries_read = 0;
    
    // We need to parse the ring buffer for structured entries
    // The ring buffer contains LogEntryInternal structures
    // We'll iterate through the buffer and count/extract matching entries
    
    if (ring_bytes == 0) {
        return 0;
    }
    
    // Walk through the ring buffer looking for complete entries
    std::size_t scan = ring_bytes;
    std::size_t cursor = ring_read;
    std::size_t entry_size = sizeof(LogEntryInternal);
    
    // Since we write structured entries, we can read them directly
    // But we need to handle the case where entries wrap around
    // For simplicity, we'll read from the current read position
    
    while (entries_read < count && scan >= entry_size) {
        // Check if we have a complete entry at this position
        LogEntryInternal* entry = reinterpret_cast<LogEntryInternal*>(&ring_buffer[cursor]);
        
        // Validate the entry (basic sanity check)
        if (entry->level <= static_cast<std::uint8_t>(Level::DEBUG)) {
            // Check level filter
            if (level_filter < 0 || entry->level <= static_cast<std::uint8_t>(level_filter)) {
                if (!just_count) {
                    entries[entries_read].level = entry->level;
                    entries[entries_read].timestamp_sec = entry->timestamp_sec;
                    entries[entries_read].timestamp_nsec = entry->timestamp_nsec;
                    std::strncpy(entries[entries_read].message, entry->message, sizeof(entries[0].message) - 1);
                    entries[entries_read].message[sizeof(entries[0].message) - 1] = '\0';
                }
                ++entries_read;
            }
        }
        
        // Move to next entry
        cursor = (cursor + entry_size) % RING_SIZE;
        if (scan >= entry_size) {
            scan -= entry_size;
        } else {
            scan = 0;
        }
    }
    
    return entries_read;
}

void syslog_clear() noexcept {
    ring_write = 0;
    ring_read = 0;
    ring_bytes = 0;
}

LogStats syslog_stats() noexcept {
    return {total_written, dropped, RING_SIZE, ring_bytes};
}

} // namespace kernel::log