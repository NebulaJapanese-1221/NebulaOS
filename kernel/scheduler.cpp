// Scheduler Implementation for NebulaOS
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

#include "scheduler.hpp"
#include "pmm.hpp"
#include "heap.hpp"
#include "timer.hpp"
#include <cstring>

namespace kernel::scheduler {

namespace {

// Thread list
Thread* thread_list = nullptr;
Thread* current = nullptr;
Thread* idle_thread = nullptr;

// Ready queue (doubly linked)
Thread* ready_head = nullptr;
Thread* ready_tail = nullptr;

// Wait queues (simple linked list of threads)
struct WaitQueue {
    Thread* head;
    Thread* tail;
};

// Statistics
SchedulerStats scheduler_stats = {};

// Time slice
std::uint32_t current_time_slice = DEFAULT_TIME_SLICE_MS;

// Exit handler
ExitHandler exit_handler = nullptr;

// Next thread ID
std::uint32_t next_thread_id = 1;

// Preemption state
bool preemption_enabled = true;

// Allocate a kernel stack for a thread
std::uintptr_t allocate_kernel_stack(std::size_t size) {
    const std::size_t pages = (size + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;
    const auto frame = pmm::allocate_frames(pages, pmm::FrameFlags::ZEROED);
    if (!frame.ok) {
        return 0;
    }
    return frame.value;
}

// Allocate a user stack for a thread
std::uintptr_t allocate_user_stack(std::size_t size) {
    const std::size_t pages = (size + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;
    const auto frame = pmm::allocate_frames(pages, pmm::FrameFlags::ZEROED);
    if (!frame.ok) {
        return 0;
    }
    return frame.value;
}

// Add a thread to the ready queue
void enqueue_ready(Thread* thread) {
    thread->state = ThreadState::READY;
    thread->prev = ready_tail;
    thread->next = nullptr;

    if (ready_tail != nullptr) {
        ready_tail->next = thread;
    } else {
        ready_head = thread;
    }
    ready_tail = thread;
}

// Remove a thread from the ready queue
void dequeue_ready(Thread* thread) {
    if (thread->prev != nullptr) {
        thread->prev->next = thread->next;
    } else {
        ready_head = thread->next;
    }
    if (thread->next != nullptr) {
        thread->next->prev = thread->prev;
    } else {
        ready_tail = thread->prev;
    }
    thread->prev = nullptr;
    thread->next = nullptr;
}

// Remove a thread from the global list
void remove_from_list(Thread* thread) {
    if (thread->prev != nullptr) {
        thread->prev->next = thread->next;
    } else {
        thread_list = thread->next;
    }
    if (thread->next != nullptr) {
        thread->next->prev = thread->prev;
    }
}

// The idle thread - runs when no other thread is ready
void idle_entry() {
    for (;;) {
        asm volatile("hlt");
    }
}

// Thread entry wrapper
void thread_entry_wrapper(Thread* thread, void (*entry)(void*), void* arg) {
    // Set up the thread's state
    thread->state = ThreadState::RUNNING;

    // Call the entry point
    entry(arg);

    // If we get here, the thread function returned
    thread->state = ThreadState::DEAD;
    thread->exit_code = 0;

    if (exit_handler != nullptr) {
        exit_handler(thread);
    }

    // Yield to another thread
    yield();

    // Should never reach here
    for (;;) {
        asm volatile("hlt");
    }
}

} // namespace

bool initialize() {
    thread_list = nullptr;
    current = nullptr;
    idle_thread = nullptr;
    ready_head = nullptr;
    ready_tail = nullptr;
    next_thread_id = 1;

    scheduler_stats = {};
    current_time_slice = DEFAULT_TIME_SLICE_MS;
    exit_handler = nullptr;
    preemption_enabled = true;

    // Create the idle thread
    idle_thread = create_thread(idle_entry, nullptr,
                                  PRIORITY_MAX,
                                  SchedulingPolicy::FIFO);
    if (idle_thread == nullptr) {
        return false;
    }

    return true;
}

[[noreturn]] void start() {
    if (ready_head == nullptr) {
        // No threads to run, just halt
        for (;;) {
            asm volatile("hlt");
        }
    }

    // Switch to the first ready thread
    Thread* first = ready_head;
    current = first;
    current->state = ThreadState::RUNNING;
    current->time_slice = current_time_slice;

    // Context switch to the first thread
    context_switch(nullptr, first);

    // Should never reach here
    for (;;) {
        asm volatile("hlt");
    }
}

void yield() {
    if (current == nullptr) {
        return;
    }

    // Save current state
    Thread* old = current;

    // Find the next thread to run
    Thread* next = next_thread();
    if (next == nullptr || next == old) {
        return;
    }

    // Update statistics
    ++scheduler_stats.context_switches;

    // Switch
    current = next;
    next->state = ThreadState::RUNNING;
    next->time_slice = current_time_slice;

    if (old->state == ThreadState::RUNNING) {
        old->state = ThreadState::READY;
        enqueue_ready(old);
    }

    context_switch(old, next);
}

// Called from the timer interrupt to implement preemptive multitasking.
// Decrements the current thread's time slice and forces a yield when expired.
void tick() {
    if (current == nullptr) {
        return;
    }

    // Update statistics
    ++scheduler_stats.system_time;

    // Only preempt if preemption is enabled and we're not in the idle thread
    if (!preemption_enabled || current == idle_thread) {
        return;
    }

    if (current->time_slice > 0) {
        --current->time_slice;
        if (current->time_slice == 0) {
            // Time slice expired - force yield
            yield();
        }
    }
}

void schedule(Thread* thread) {
    if (thread == nullptr || thread->state == ThreadState::RUNNING) {
        return;
    }

    // Remove from current position
    if (thread->prev != nullptr || thread->next != nullptr) {
        if (thread == ready_head) {
            dequeue_ready(thread);
        }
    }

    enqueue_ready(thread);
}

void block(void* wait_queue) {
    if (current == nullptr) {
        return;
    }

    current->state = ThreadState::BLOCKED;
    current->wait_queue = wait_queue;

    // Remove from ready queue
    if (current->prev != nullptr || current->next != nullptr) {
        dequeue_ready(current);
    }

    // Find next thread
    Thread* next = next_thread();
    if (next == nullptr) {
        return;
    }

    ++scheduler_stats.context_switches;
    current = next;
    next->state = ThreadState::RUNNING;
    next->time_slice = current_time_slice;

    context_switch(nullptr, next);
}

void unblock(Thread* thread) {
    if (thread == nullptr || thread->state != ThreadState::BLOCKED) {
        return;
    }

    thread->state = ThreadState::READY;
    thread->wait_queue = nullptr;
    enqueue_ready(thread);
}

void sleep(std::uint32_t milliseconds) {
    if (current == nullptr) {
        return;
    }

    // Set the thread to sleep
    current->state = ThreadState::SLEEPING;
    current->wait_queue = nullptr;

    // Remove from ready queue
    if (current->prev != nullptr || current->next != nullptr) {
        dequeue_ready(current);
    }

    // Find next thread
    Thread* next = next_thread();
    if (next == nullptr) {
        return;
    }

    ++scheduler_stats.context_switches;
    current = next;
    next->state = ThreadState::RUNNING;
    next->time_slice = current_time_slice;

    context_switch(nullptr, next);
}

Thread* create_thread(void (*entry)(void*), void* arg,
                       std::uint8_t priority,
                       SchedulingPolicy policy) {
    if (entry == nullptr) {
        return nullptr;
    }

    // Allocate thread structure
    Thread* thread = static_cast<Thread*>(
        heap::allocate(sizeof(Thread)));
    if (thread == nullptr) {
        return nullptr;
    }

    std::memset(thread, 0, sizeof(Thread));

    // Allocate kernel stack (8 KiB + 1 guard page)
    const std::size_t kernel_stack_size = 8 * 1024;
    const std::size_t guard_page_size = pmm::PAGE_SIZE;
    const std::size_t total_stack_pages = (kernel_stack_size + guard_page_size + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;
    const auto frame = pmm::allocate_frames(total_stack_pages, pmm::FrameFlags::ZEROED);
    if (!frame.ok) {
        heap::release(thread);
        return nullptr;
    }

    std::uintptr_t stack_base = frame.value;
    std::uintptr_t guard_page = stack_base;
    std::uintptr_t kernel_stack_actual = stack_base + guard_page_size;

    // Allocate user stack (8 KiB)
    const std::size_t user_stack_size = 8 * 1024;
    const std::size_t user_stack_pages = (user_stack_size + pmm::PAGE_SIZE - 1) / pmm::PAGE_SIZE;
    const auto user_frame = pmm::allocate_frames(user_stack_pages, pmm::FrameFlags::ZEROED);
    if (!user_frame.ok) {
        pmm::free_frames(stack_base, total_stack_pages);
        heap::release(thread);
        return nullptr;
    }

    // Initialize thread
    thread->id = next_thread_id++;
    thread->state = ThreadState::CREATED;
    thread->policy = policy;
    thread->priority = priority;
    thread->time_slice = current_time_slice;
    thread->total_time = 0;

    thread->kernel_stack_base = kernel_stack_actual;
    thread->kernel_stack_top = kernel_stack_actual + kernel_stack_size;
    thread->user_stack_base = user_frame.value;
    thread->user_stack_top = user_frame.value + user_stack_size;
    thread->kernel_guard_page = guard_page;

    thread->process_id = 0;
    thread->next = nullptr;
    thread->prev = nullptr;
    thread->wait_queue = nullptr;
    thread->exit_code = 0;
    thread->fpu_used = false;

    // Set up initial context
    std::memset(&thread->context, 0, sizeof(ThreadContext));

    // Set up stack for the thread entry
    // The stack grows downward, so push arguments at the top
    std::uint32_t* stack = reinterpret_cast<std::uint32_t*>(
        thread->kernel_stack_top);

    // Push argument
    *--stack = reinterpret_cast<std::uint32_t>(arg);

    // Push return address (thread_entry_wrapper)
    *--stack = reinterpret_cast<std::uint32_t>(thread_entry_wrapper);

    // Set initial ESP
    thread->context.esp = reinterpret_cast<std::uint32_t>(stack);
    thread->context.eip = reinterpret_cast<std::uint32_t>(thread_entry_wrapper);

    // Set segment registers
    thread->context.ds = 0x10;  // Kernel data segment
    thread->context.es = 0x10;
    thread->context.fs = 0x10;
    thread->context.gs = 0x10;

    // Set flags (interrupts enabled)
    thread->context.eflags = 0x202;

    // Add to global list
    thread->next = thread_list;
    if (thread_list != nullptr) {
        thread_list->prev = thread;
    }
    thread_list = thread;

    ++scheduler_stats.total_threads_created;

    // Add to ready queue
    enqueue_ready(thread);

    return thread;
}

void destroy_thread(Thread* thread) {
    if (thread == nullptr) {
        return;
    }

    // Remove from list
    remove_from_list(thread);

    // Free stacks
    const std::size_t kernel_pages = (thread->kernel_stack_top - thread->kernel_stack_base) / pmm::PAGE_SIZE;
    const std::size_t guard_pages = pmm::PAGE_SIZE / pmm::PAGE_SIZE;  // 1 page
    const std::size_t user_pages = (thread->user_stack_top - thread->user_stack_base) / pmm::PAGE_SIZE;

    pmm::free_frames(thread->kernel_stack_base, kernel_pages);
    pmm::free_frames(thread->kernel_guard_page, guard_pages);
    pmm::free_frames(thread->user_stack_base, user_pages);

    // Free thread structure
    heap::release(thread);

    ++scheduler_stats.total_threads_destroyed;
}

Thread* current_thread() noexcept {
    return current;
}

Thread* next_thread() noexcept {
    if (ready_head == nullptr) {
        return idle_thread;
    }

    // Simple round-robin: take the first ready thread
    Thread* next = ready_head;

    // If priority scheduling, find the highest priority thread
    if (next != nullptr) {
        for (Thread* t = ready_head; t != nullptr; t = t->next) {
            if (t->priority < next->priority) {
                next = t;
            }
        }
    }

    return next;
}

SchedulerStats stats() noexcept {
    return scheduler_stats;
}

void set_time_slice(std::uint32_t milliseconds) {
    current_time_slice = milliseconds;
}

std::uint32_t time_slice() noexcept {
    return current_time_slice;
}

void set_exit_handler(ExitHandler handler) {
    exit_handler = handler;
}

bool set_preemption(bool enabled) noexcept {
    bool old = preemption_enabled;
    preemption_enabled = enabled;
    return old;
}

bool preempt_enabled() noexcept {
    return preemption_enabled;
}

// FPU exception handler (called from assembly for vector 7 - Device Not Available)
// This implements lazy FPU context switching.
extern "C" void scheduler_fpu_exception() {
    // Save the old thread's FPU state if it was using FPU
    if (current != nullptr && current->fpu_used) {
        asm volatile("fxsave %0" : : "m"(current->fpu_state));
    }
    
    // Enable FPU for the new thread
    if (current != nullptr) {
        current->fpu_used = true;
        
        // Restore FPU state if this thread has used FPU before
        asm volatile("fxrstor %0" : : "m"(current->fpu_state));
    }
    
    // Clear CR0.TS (Task Switched) bit to allow FPU instructions
    asm volatile("mov %%cr0, %%eax; and $~0x8, %%eax; mov %%eax, %%cr0" : : : "eax", "memory");
}

} // namespace kernel::scheduler