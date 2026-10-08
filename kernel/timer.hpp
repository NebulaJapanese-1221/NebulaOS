// Timer subsystem for NebulaOS
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

namespace kernel::timer {

// The PIT is programmed from this rate, so a hundred ticks make a second.
constexpr std::uint32_t ticks_per_second = 100;

// Nanoseconds per second
constexpr std::uint64_t nanoseconds_per_second = 1000000000ULL;

// Clock sources available on the system
enum class ClockSource : std::uint8_t {
    PIT = 0,   // Programmable Interval Timer
    TSC = 1,   // Time Stamp Counter
    HPET = 2,  // High Precision Event Timer
    ACPI = 3,  // ACPI PM Timer
};

// Initialize the timer subsystem
void initialize();

// Called from the timer interrupt handler
void interrupt_tick();

// ---- Tick-based time (legacy API) ----

// Number of ticks since boot
std::uint32_t ticks();

// Whole seconds since boot, derived from ticks()
std::uint32_t seconds();

// Sleep for the given number of seconds (tick-based)
void sleep_seconds(std::uint32_t seconds);

// Sleep for the given number of milliseconds (tick-based)
void sleep_ms(std::uint32_t milliseconds);

// Sleep for the given number of microseconds (tick-based, approximate)
void sleep_us(std::uint32_t microseconds);

// ---- High-resolution time (TSC-based) ----

// Initialize the TSC clock source
bool initialize_tsc();

// Read the time stamp counter
std::uint64_t read_tsc();

// Get the TSC frequency in Hz
std::uint64_t tsc_frequency();

// Get time in nanoseconds (TSC-based)
std::uint64_t nanoseconds();

// Get time in microseconds (TSC-based)
std::uint64_t microseconds();

// Get time in milliseconds (TSC-based)
std::uint64_t milliseconds();

// High-resolution delay in nanoseconds (busy-wait)
void delay_ns(std::uint64_t nanoseconds);

// High-resolution delay in microseconds (busy-wait)
void delay_us(std::uint64_t microseconds);

// Hardware timed wait that needs neither the PIT interrupt nor the timer to be
// running, because it counts through a spare PIT channel and polls for the end
// of the count. sleep_seconds() cannot be used during boot, since it waits for
// the interrupts that have not been installed yet and would never return.
void delay_ms(std::uint32_t milliseconds);

// ---- Monotonic clock ----

// Get the monotonic clock value in nanoseconds
std::uint64_t monotonic_ns();

// Get the boot time in seconds since epoch (if available)
std::uint64_t boot_time();

// ---- Timer statistics ----

struct TimerStats {
    std::uint64_t total_ticks;
    std::uint32_t ticks_per_second;
    std::uint64_t tsc_frequency;
    ClockSource source;
    bool tsc_available;
    bool invariant_tsc;
};

// Get timer statistics
TimerStats stats();

// Set the clock source
bool set_clock_source(ClockSource source);

// Get the current clock source
ClockSource current_clock_source();

// ---- Timer callbacks (for future scheduling integration) ----

using TimerCallback = void (*)(void* data);

// Register a periodic timer callback
int register_timer(std::uint32_t interval_ms, TimerCallback callback,
                        void* data);

// Unregister a timer callback
void unregister_timer(int timer_id);

} // namespace kernel::timer