// Scheduler for NebulaOS
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

namespace kernel::scheduler {

// Thread states
enum class ThreadState : std::uint8_t {
    CREATED   = 0,  // Created but not yet run
    READY     = 1,  // Ready to run
    RUNNING   = 2,  // Currently running
    BLOCKED   = 3,  // Waiting for an event
    SLEEPING  = 4,  // Waiting for a timeout
    DEAD      = 5,  // Terminated
};

// Scheduling policies
enum class SchedulingPolicy : std::uint8_t {
    ROUND_ROBIN   = 0,  // Time-sliced round-robin
    FIFO          = 1,  // First in, first out
    PRIORITY      = 2,  // Priority-based
};

// Thread priority (0 = highest, 255 = lowest)
constexpr std::uint8_t PRIORITY_MIN = 0;
constexpr std::uint8_t PRIORITY_NORMAL = 128;
constexpr std::uint8_t PRIORITY_MAX = 255;

// Default time slice in milliseconds
constexpr std::uint32_t DEFAULT_TIME_SLICE_MS = 10;

// Thread context (saved during context switch)
struct ThreadContext {
    // General purpose registers
    std::uint32_t edi;
    std::uint32_t esi;
    std::uint32_t ebp;
    std::uint32_t esp;
    std::uint32_t ebx;
    std::uint32_t edx;
    std::uint32_t ecx;
    std::uint32_t eax;

    // Segment selectors
    std::uint32_t ds;
    std::uint32_t es;
    std::uint32_t fs;
    std::uint32_t gs;

    // Control registers (for FPU/SSE state)
    std::uint32_t cr3;  // Page directory base

    // Stack pointers
    std::uint32_t kernel_esp;
    std::uint32_t user_esp;

    // Instruction pointer and flags
    std::uint32_t eip;
    std::uint32_t eflags;
};

// Thread structure
struct Thread {
    std::uint32_t id;
    ThreadState state;
    SchedulingPolicy policy;
    std::uint8_t priority;
    std::uint32_t time_slice;  // Remaining time in current slice
    std::uint32_t total_time;  // Total execution time in ticks

    // Context
    ThreadContext context;

    // Stack information
    std::uintptr_t kernel_stack_base;
    std::uintptr_t kernel_stack_top;
    std::uintptr_t user_stack_base;
    std::uintptr_t user_stack_top;

    // Process affiliation
    std::uint32_t process_id;

    // Linked list pointers
    Thread* next;
    Thread* prev;

    // Wait queue (for blocked threads)
    void* wait_queue;

    // Exit status
    int exit_code;
};

// Scheduler statistics
struct SchedulerStats {
    std::uint32_t context_switches;
    std::uint32_t total_threads_created;
    std::uint32_t total_threads_destroyed;
    std::uint32_t idle_time;
    std::uint32_t system_time;
};

// Initialize the scheduler
bool initialize();

// Start the scheduler (never returns)
[[noreturn]] void start();

// Yield the current thread
void yield();

// Schedule a thread (mark as ready)
void schedule(Thread* thread);

// Block the current thread on a wait queue
void block(void* wait_queue);

// Unblock a thread from a wait queue
void unblock(Thread* thread);

// Put the current thread to sleep
void sleep(std::uint32_t milliseconds);

// Create a new thread
Thread* create_thread(void (*entry)(void*), void* arg,
                       std::uint8_t priority = PRIORITY_NORMAL,
                       SchedulingPolicy policy = SchedulingPolicy::ROUND_ROBIN);

// Destroy a thread
void destroy_thread(Thread* thread);

// Get the current thread
Thread* current_thread() noexcept;

// Get the next thread to run
Thread* next_thread() noexcept;

// Context switch (assembly)
extern "C" void context_switch(Thread* old_thread, Thread* new_thread);

// Get scheduler statistics
SchedulerStats stats() noexcept;

// Set the time slice for the scheduler
void set_time_slice(std::uint32_t milliseconds);

// Get the current time slice
std::uint32_t time_slice() noexcept;

// Register a thread exit handler
using ExitHandler = void (*)(Thread*);
void set_exit_handler(ExitHandler handler);

} // namespace kernel::scheduler