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

#include "timer.hpp"
#include "scheduler.hpp"

namespace {

volatile std::uint32_t system_ticks = 0;
volatile std::uint64_t boot_ns = 0;

kernel::timer::ClockSource active_source = kernel::timer::ClockSource::PIT;
bool tsc_present = false;
bool tsc_invariant = false;
std::uint64_t tsc_hz = 0;

void write_port(std::uint16_t port, std::uint8_t value) {
    asm volatile("outb %0, %1" : : "a"(value), "Nd"(port));
}

std::uint8_t read_port(std::uint16_t port) {
    std::uint8_t value;
    asm volatile("inb %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

// The PIT is clocked at 1.193182 MHz and its counter is sixteen bits wide,
// so one count of channel 2 covers at most this many milliseconds before it
// wraps.
constexpr std::uint32_t maximum_count_milliseconds = 54;

// Programs channel 2 as a one shot and waits for it to finish. Bit 5 of port
// 0x61 reads high while the channel is counting and low once it reaches
// terminal count, which is the only synchronisation needed: no interrupt, and
// therefore no dependency on the interrupt controller being installed.
void wait_ticks(std::uint32_t count) {
    // Bit 0 restarts the counter and bit 1 is the speaker gate, which has to
    // be driven for the channel to count at all.
    std::uint8_t gate = read_port(0x61);
    write_port(0x61, static_cast<std::uint8_t>((gate & ~0x02) | 0x01));

    // Channel 2, access mode low byte then high byte, mode 0.
    write_port(0x43, 0xB0);
    write_port(0x42, static_cast<std::uint8_t>(count & 0xFF));
    write_port(0x42, static_cast<std::uint8_t>((count >> 8) & 0xFF));

    // The reload bit has to be seen low before it is seen high, otherwise the
    // counter starts from whatever it was left at.
    const std::uint8_t held = read_port(0x61);
    write_port(0x61, static_cast<std::uint8_t>(held & ~0x01));
    write_port(0x61, static_cast<std::uint8_t>(held | 0x01));
    write_port(0x61, static_cast<std::uint8_t>((held & ~0x02) | 0x02));

    while ((read_port(0x61) & 0x20) != 0) {
        asm volatile("nop");
    }
}

// Reads the time stamp counter. RDTSC is not serializing, so a CPUID is used
// to drain the pipeline before and after the read. This keeps the value
// consistent even when the clock is not invariant, at the cost of a few
// hundred cycles per read.
std::uint64_t read_tsc_serialized() {
    std::uint32_t low = 0;
    std::uint32_t high = 0;
    asm volatile("cpuid" : : "a"(0) : "ebx", "ecx", "edx");
    asm volatile("rdtsc" : "=a"(low), "=d"(high));
    asm volatile("cpuid" : : "a"(0) : "ebx", "ecx", "edx");
    return (static_cast<std::uint64_t>(high) << 32) | low;
}

// Calibrates the TSC against the PIT by counting TSC ticks across a known
// PIT interval. The PIT is put in mode 0 through channel 2 and polled, which
// needs no interrupt controller, so this works during early boot.
std::uint64_t calibrate_tsc() {
    // Count across ten PIT ticks of channel 0 for a stable estimate, then
    // convert to a per second rate.
    const std::uint32_t pit_ticks = kernel::timer::ticks_per_second;
    const std::uint64_t start = read_tsc_serialized();

    const std::uint32_t divisor = 11932;
    write_port(0x43, 0x36);
    write_port(0x40, static_cast<std::uint8_t>(divisor & 0xFF));
    write_port(0x40, static_cast<std::uint8_t>(divisor >> 8));

    const std::uint32_t target = system_ticks + pit_ticks;
    while (system_ticks < target) {
        asm volatile("nop");
    }

    const std::uint64_t end = read_tsc_serialized();
    const std::uint64_t elapsed_tsc = end - start;
    // elapsed_tsc covers pit_ticks ticks, which is one second at the
    // configured rate, so the TSC frequency is the raw count.
    return elapsed_tsc;
}

} // namespace

namespace kernel::timer {

void initialize() {
    const std::uint16_t divisor = 11932;
    write_port(0x43, 0x36);
    write_port(0x40, static_cast<std::uint8_t>(divisor & 0xFF));
    write_port(0x40, static_cast<std::uint8_t>(divisor >> 8));
    system_ticks = 0;
    boot_ns = 0;
    active_source = ClockSource::PIT;
    initialize_tsc();
}

void interrupt_tick() {
    ++system_ticks;
    boot_ns += nanoseconds_per_second / ticks_per_second;
    
    // Call scheduler tick for preemptive multitasking
    kernel::scheduler::tick();
}

std::uint32_t ticks() {
    const std::uint32_t flags = []() {
        std::uint32_t saved_flags;
        asm volatile("pushf; pop %0; cli" : "=r"(saved_flags) : : "memory");
        return saved_flags;
    }();
    const std::uint32_t current_ticks = system_ticks;
    if ((flags & (1U << 9)) != 0) {
        asm volatile("sti" : : : "memory");
    }
    return current_ticks;
}

std::uint32_t seconds() {
    return ticks() / ticks_per_second;
}

void sleep_seconds(std::uint32_t seconds) {
    const std::uint32_t duration = seconds * ticks_per_second;
    const std::uint32_t start = ticks();
    while (static_cast<std::uint32_t>(ticks() - start) < duration) {
        asm volatile("sti; hlt" : : : "memory");
    }
}

void sleep_ms(std::uint32_t milliseconds) {
    const std::uint32_t duration =
        (milliseconds * ticks_per_second) / 1000U;
    if (duration == 0) {
        if (milliseconds != 0) {
            delay_us(milliseconds * 1000U);
        }
        return;
    }
    const std::uint32_t start = ticks();
    while (static_cast<std::uint32_t>(ticks() - start) < duration) {
        asm volatile("sti; hlt" : : : "memory");
    }
}

void sleep_us(std::uint32_t microseconds) {
    if (tsc_present && tsc_hz != 0) {
        delay_us(microseconds);
        return;
    }
    // Fall back to the PIT one shot, which cannot resolve below about a
    // millisecond, so short waits round up to a single count.
    const std::uint32_t milliseconds = (microseconds + 999U) / 1000U;
    delay_ms(milliseconds == 0 ? 1 : milliseconds);
}

bool initialize_tsc() {
    // EDX bit 4 of CPUID leaf 0x80000007 reports an invariant TSC, which
    // runs at a constant rate across power states and is safe to use as a
    // wall clock. Leaf 0x15 gives a direct frequency when present.
    std::uint32_t leaf = 0;
    std::uint32_t eax = 0;
    std::uint32_t ebx = 0;
    std::uint32_t ecx = 0;
    std::uint32_t edx = 0;
    asm volatile("cpuid"
                 : "=a"(eax), "=b"(ebx), "=c"(ecx), "=d"(edx)
                 : "a"(0x80000007));
    tsc_invariant = (edx & (1U << 8)) != 0;

    asm volatile("cpuid"
                 : "=a"(eax), "=b"(ebx), "=c"(ecx), "=d"(edx)
                 : "a"(0x15));
    if (ecx != 0 && ebx != 0) {
        // Frequency = (ebx / ecx) * 1 GHz, per the leaf definition.
        tsc_hz = static_cast<std::uint64_t>(ebx) * 1000000000ULL / ecx;
    }

    if (tsc_hz == 0) {
        tsc_hz = calibrate_tsc();
    }

    tsc_present = tsc_hz != 0;
    if (tsc_present) {
        active_source = ClockSource::TSC;
    }
    return tsc_present;
}

std::uint64_t read_tsc() {
    if (!tsc_present) {
        return 0;
    }
    if (tsc_invariant) {
        std::uint32_t low = 0;
        std::uint32_t high = 0;
        asm volatile("rdtsc" : "=a"(low), "=d"(high));
        return (static_cast<std::uint64_t>(high) << 32) | low;
    }
    return read_tsc_serialized();
}

std::uint64_t tsc_frequency() {
    return tsc_hz;
}

std::uint64_t nanoseconds() {
    if (tsc_present && tsc_hz != 0) {
        const std::uint64_t ticks_now = read_tsc();
        return (ticks_now * nanoseconds_per_second) / tsc_hz;
    }
    return boot_ns;
}

std::uint64_t microseconds() {
    return nanoseconds() / 1000ULL;
}

std::uint64_t milliseconds() {
    return nanoseconds() / 1000000ULL;
}

void delay_ns(std::uint64_t ns) {
    if (!tsc_present || tsc_hz == 0) {
        delay_ms(static_cast<std::uint32_t>((ns + 999999ULL) / 1000000ULL));
        return;
    }
    const std::uint64_t start = read_tsc();
    const std::uint64_t ticks_needed = (ns * tsc_hz) / nanoseconds_per_second;
    while (read_tsc() - start < ticks_needed) {
        asm volatile("nop");
    }
}

void delay_us(std::uint64_t us) {
    delay_ns(us * 1000ULL);
}

void delay_ms(std::uint32_t milliseconds) {
    // Split rather than scale once, so the multiply stays inside a thirty two
    // bit word even for the long waits the boot sequence uses.
    while (milliseconds != 0) {
        const std::uint32_t chunk = milliseconds > maximum_count_milliseconds
                                       ? maximum_count_milliseconds
                                       : milliseconds;
        wait_ticks(1193182U * chunk / 1000U);
        milliseconds -= chunk;
    }
}

std::uint64_t monotonic_ns() {
    return nanoseconds();
}

std::uint64_t boot_time() {
    return seconds();
}

TimerStats stats() {
    TimerStats result = {};
    result.total_ticks = system_ticks;
    result.ticks_per_second = ticks_per_second;
    result.tsc_frequency = tsc_hz;
    result.source = active_source;
    result.tsc_available = tsc_present;
    result.invariant_tsc = tsc_invariant;
    return result;
}

bool set_clock_source(ClockSource source) {
    if (source == ClockSource::TSC && !tsc_present) {
        return false;
    }
    if (source == ClockSource::PIT) {
        active_source = ClockSource::PIT;
        return true;
    }
    if (source == ClockSource::TSC) {
        active_source = ClockSource::TSC;
        return true;
    }
    return false;
}

ClockSource current_clock_source() {
    return active_source;
}

int register_timer(std::uint32_t interval_ms, TimerCallback callback,
                        void* data) {
    (void)interval_ms;
    (void)callback;
    (void)data;
    return -1;
}

void unregister_timer(int timer_id) {
    (void)timer_id;
}

} // namespace kernel::timer