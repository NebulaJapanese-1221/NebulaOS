// Process Management Implementation for NebulaOS
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

#include "process.hpp"
#include "heap.hpp"
#include "pmm.hpp"
#include "../drivers/serial.hpp"
#include <cstring>

namespace kernel::process {

namespace {

// Process list
Process* process_list = nullptr;
std::uint32_t process_count = 0;

// Current process
Process* current_process = nullptr;

// Next process ID
std::uint32_t next_process_id = 1;

// Exit handler
ExitHandler exit_handler = nullptr;

// Process names (stored in the process structure)
// For simplicity, we'll use a fixed-size buffer

// Find a free slot in the process list
Process* find_free_slot() {
    // In a more sophisticated implementation, we'd have a free list
    // For now, just check the linked list
    return nullptr;
}

} // namespace

bool initialize() {
    process_list = nullptr;
    process_count = 0;
    current_process = nullptr;
    next_process_id = 1;
    exit_handler = nullptr;

    // Create the init process (PID 1)
    Process* init = static_cast<Process*>(
        heap::allocate(sizeof(Process)));
    if (init == nullptr) {
        return false;
    }

    std::memset(init, 0, sizeof(Process));
    init->id = 0;  // Kernel process
    init->state = ProcessState::RUNNING;
    init->flags = FLAG_SYSTEM | FLAG_KERNEL;
    init->address_space.page_directory_phys = 0;
    init->address_space.kernel_cr3 = 0;
    init->thread_count = 0;
    init->main_thread = nullptr;
    init->parent_id = 0;
    init->child_count = 0;
    init->fd_count = 0;
    init->message_port = nullptr;
    init->exit_code = 0;
    init->exited = false;
    init->next = nullptr;
    init->prev = nullptr;

    std::strcpy(init->working_directory, "/");

    process_list = init;
    current_process = init;
    ++process_count;

    return true;
}

Process* create(const CreateInfo& info) {
    if (info.entry == nullptr) {
        return nullptr;
    }

    // Allocate process structure
    Process* process = static_cast<Process*>(
        heap::allocate(sizeof(Process)));
    if (process == nullptr) {
        return nullptr;
    }

    std::memset(process, 0, sizeof(Process));

    // Create address space
    auto space_result = paging::create_address_space();
    if (!space_result.ok) {
        heap::release(process);
        return nullptr;
    }
    process->address_space = space_result.value;

    // Create main thread
    scheduler::Thread* thread = scheduler::create_thread(
        info.entry, info.arg,
        info.priority,
        scheduler::SchedulingPolicy::ROUND_ROBIN);
    if (thread == nullptr) {
        paging::destroy_address_space(process->address_space);
        heap::release(process);
        return nullptr;
    }

    // Initialize process
    process->id = next_process_id++;
    process->state = ProcessState::READY;
    process->flags = info.flags;
    process->threads[0] = thread;
    process->thread_count = 1;
    process->main_thread = thread;
    process->parent_id = current_process != nullptr ? current_process->id : 0;
    process->child_count = 0;
    process->fd_count = 0;
    process->message_port = ipc::create_port(64);
    process->exit_code = 0;
    process->exited = false;

    std::strcpy(process->working_directory, "/");

    // Set thread's process ID
    thread->process_id = process->id;

    // Add to global list
    process->next = process_list;
    if (process_list != nullptr) {
        process_list->prev = process;
    }
    process_list = process;
    ++process_count;

    // Add to parent's children
    if (current_process != nullptr &&
        current_process->child_count < MAX_PROCESSES) {
        current_process->child_ids[current_process->child_count++] = process->id;
    }

    return process;
}

Process* create_kernel(void (*entry)(void*), void* arg,
                         std::uint32_t flags) {
    CreateInfo info;
    info.name = "kernel";
    info.entry = entry;
    info.arg = arg;
    info.flags = flags | FLAG_KERNEL | FLAG_SYSTEM;
    info.priority = scheduler::PRIORITY_NORMAL;

    return create(info);
}

void destroy(Process* process) {
    if (process == nullptr) {
        return;
    }

    // Remove from global list
    if (process_list == process) {
        process_list = process->next;
    } else {
        for (Process* p = process_list; p != nullptr; p = p->next) {
            if (p->next == process) {
                p->next = process->next;
                break;
            }
        }
    }

    // Destroy threads
    for (std::size_t i = 0; i < process->thread_count; ++i) {
        if (process->threads[i] != nullptr) {
            scheduler::destroy_thread(process->threads[i]);
        }
    }

    // Destroy address space
    paging::destroy_address_space(process->address_space);

    // Destroy message port
    if (process->message_port != nullptr) {
        ipc::destroy_port(process->message_port);
    }

    // Remove from parent's children
    if (process->parent_id != 0) {
        Process* parent = get_by_id(process->parent_id);
        if (parent != nullptr) {
            for (std::size_t i = 0; i < parent->child_count; ++i) {
                if (parent->child_ids[i] == process->id) {
                    // Shift remaining children
                    for (std::size_t j = i; j < parent->child_count - 1; ++j) {
                        parent->child_ids[j] = parent->child_ids[j + 1];
                    }
                    --parent->child_count;
                    break;
                }
            }
        }
    }

    heap::release(process);
    --process_count;
}

[[noreturn]] void exit(int code) {
    if (current_process == nullptr) {
        // No process, just halt
        for (;;) {
            asm volatile("cli; hlt");
        }
    }

    current_process->state = ProcessState::ZOMBIE;
    current_process->exit_code = code;
    current_process->exited = true;

    // Notify parent
    if (exit_handler != nullptr) {
        exit_handler(current_process);
    }

    // Yield to another process
    scheduler::yield();

    // Should never reach here
    for (;;) {
        asm volatile("cli; hlt");
    }
}

int wait(std::uint32_t process_id, int* out_code) {
    if (current_process == nullptr) {
        return -1;
    }

    // Find the child process
    bool found = false;
    for (std::size_t i = 0; i < current_process->child_count; ++i) {
        if (current_process->child_ids[i] == process_id) {
            found = true;
            break;
        }
    }

    if (!found) {
        return -1;
    }

    Process* child = get_by_id(process_id);
    if (child == nullptr) {
        return -1;
    }

    // Wait for the child to exit
    while (!child->exited) {
        scheduler::yield();
    }

    if (out_code != nullptr) {
        *out_code = child->exit_code;
    }

    // Destroy the child process
    destroy(child);

    return 0;
}

Process* get_by_id(std::uint32_t id) noexcept {
    for (Process* p = process_list; p != nullptr; p = p->next) {
        if (p->id == id) {
            return p;
        }
    }
    return nullptr;
}

Process* current() noexcept {
    return current_process;
}

Process* fork() {
    if (current_process == nullptr) {
        return nullptr;
    }

    // Create a new process with the same address space
    // (copy-on-write would be ideal, but for now we duplicate)

    Process* new_process = static_cast<Process*>(
        heap::allocate(sizeof(Process)));
    if (new_process == nullptr) {
        return nullptr;
    }

    std::memcpy(new_process, current_process, sizeof(Process));

    // Create new address space
    auto space_result = paging::create_address_space();
    if (!space_result.ok) {
        heap::release(new_process);
        return nullptr;
    }
    new_process->address_space = space_result.value;

    // Create new main thread
    scheduler::Thread* thread = scheduler::create_thread(
        nullptr, nullptr,
        scheduler::PRIORITY_NORMAL,
        scheduler::SchedulingPolicy::ROUND_ROBIN);
    if (thread == nullptr) {
        paging::destroy_address_space(new_process->address_space);
        heap::release(new_process);
        return nullptr;
    }

    new_process->id = next_process_id++;
    new_process->state = ProcessState::READY;
    new_process->threads[0] = thread;
    new_process->thread_count = 1;
    new_process->main_thread = thread;
    new_process->parent_id = current_process->id;
    new_process->child_count = 0;
    new_process->exited = false;

    thread->process_id = new_process->id;

    // Add to global list
    new_process->next = process_list;
    if (process_list != nullptr) {
        process_list->prev = new_process;
    }
    process_list = new_process;
    ++process_count;

    return new_process;
}

int exec(Process* process, const char* path,
           const char* const argv[]) {
    if (process == nullptr || path == nullptr) {
        return -1;
    }

    // In a real implementation, we'd load the ELF from initrd
    // For now, just return success
    (void)argv;

    return 0;
}

int send_signal(std::uint32_t process_id, int signal) {
    (void)process_id;
    (void)signal;
    return 0;
}

ProcessStats stats() noexcept {
    ProcessStats s = {};
    s.total_processes = process_count;

    for (Process* p = process_list; p != nullptr; p = p->next) {
        switch (p->state) {
            case ProcessState::RUNNING:
                ++s.running_processes;
                break;
            case ProcessState::SLEEPING:
                ++s.sleeping_processes;
                break;
            case ProcessState::ZOMBIE:
                ++s.zombie_processes;
                break;
            default:
                break;
        }
        s.total_threads += static_cast<std::uint32_t>(p->thread_count);
    }

    return s;
}

std::size_t list(Process** out, std::size_t max) noexcept {
    if (out == nullptr || max == 0) {
        return 0;
    }

    std::size_t count = 0;
    for (Process* p = process_list; p != nullptr && count < max; p = p->next) {
        out[count++] = p;
    }

    return count;
}

const char* name(const Process* process) noexcept {
    if (process == nullptr) {
        return "";
    }
    // In a real implementation, we'd store the name
    return "process";
}

void set_name(Process* process, const char* name) {
    (void)process;
    (void)name;
}

int chdir(Process* process, const char* path) {
    if (process == nullptr || path == nullptr) {
        return -1;
    }
    std::strncpy(process->working_directory, path,
                   sizeof(process->working_directory) - 1);
    process->working_directory[sizeof(process->working_directory) - 1] = '\0';
    return 0;
}

const char* getcwd(const Process* process) noexcept {
    if (process == nullptr) {
        return "";
    }
    return process->working_directory;
}

int add_fd(Process* process, int fd) {
    if (process == nullptr || process->fd_count >= sizeof(process->file_descriptors) / sizeof(process->file_descriptors[0])) {
        return -1;
    }
    process->file_descriptors[process->fd_count++] = fd;
    return 0;
}

int remove_fd(Process* process, int fd) {
    if (process == nullptr) {
        return -1;
    }
    for (std::size_t i = 0; i < process->fd_count; ++i) {
        if (process->file_descriptors[i] == fd) {
            // Shift remaining
            for (std::size_t j = i; j < process->fd_count - 1; ++j) {
                process->file_descriptors[j] = process->file_descriptors[j + 1];
            }
            --process->fd_count;
            return 0;
        }
    }
    return -1;
}

void set_exit_handler(ExitHandler handler) {
    exit_handler = handler;
}

} // namespace kernel::process