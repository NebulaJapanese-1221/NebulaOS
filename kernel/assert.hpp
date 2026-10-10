// Kernel assertions and panic handling for NebulaOS.
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

namespace kernel::assert {

// Assertion handler type
using AssertHandler = void (*)(const char* expr, const char* file, int line,
                                const char* func, const char* msg);

// Set a custom assertion handler (for testing, etc.)
void set_assert_handler(AssertHandler handler);

// Default assertion handler - prints to log and halts
void default_assert_handler(const char* expr, const char* file, int line,
                            const char* func, const char* msg);

// Kernel panic with register dump and stack trace
struct RegisterState {
    std::uint32_t edi, esi, ebp, esp, ebx, edx, ecx, eax;
    std::uint32_t eip, cs, eflags, user_esp, user_ss;
};

// Panic with formatted message
[[noreturn]] void panic(const char* fmt, ...);

// Panic with register state (called from exception handler)
[[noreturn]] void panic_with_regs(const RegisterState* regs, const char* msg);

// Stack trace / backtrace
// max_frames: maximum number of frames to trace (0 = unlimited)
// Returns number of frames traced
int backtrace(std::uint32_t* frames, int max_frames) noexcept;

// Print stack trace to log
void print_stack_trace() noexcept;

// Kernel assert macro
#ifdef NDEBUG
#define KASSERT(expr, ...) ((void)0)
#else
#define KASSERT(expr, ...) \
    do { \
        if (!(expr)) { \
            kernel::assert::default_assert_handler(#expr, __FILE__, __LINE__, __func__, ##__VA_ARGS__); \
        } \
    } while (0)
#endif

// Kernel assert with message
#ifdef NDEBUG
#define KASSERT_MSG(expr, msg, ...) ((void)0)
#else
#define KASSERT_MSG(expr, msg, ...) \
    do { \
        if (!(expr)) { \
            kernel::assert::default_assert_handler(#expr, __FILE__, __LINE__, __func__, msg, ##__VA_ARGS__); \
        } \
    } while (0)
#endif

// Unreachable code marker
[[noreturn]] inline void unreachable(const char* file, int line) {
    default_assert_handler("unreachable", file, line, nullptr, "Execution reached unreachable code");
}

#define UNREACHABLE() kernel::assert::unreachable(__FILE__, __LINE__)

} // namespace kernel::assert