// Process Management for NebulaOS
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
#include "scheduler.hpp"
#include "ipc.hpp"
#include "paging.hpp"

namespace kernel::process {

// Maximum number of processes
constexpr std::size_t MAX_PROCESSES = 64;

// Maximum number of threads per process
constexpr std::size_t MAX_THREADS_PER_PROCESS = 32;

// Maximum arguments for exec
constexpr std::size_t MAX_ARGUMENTS = 16;

// Process states
enum class ProcessState : std::uint8_t {
    CREATED   = 0,
    READY     = 1,
    RUNNING   = 2,
    BLOCKED   = 3,
    SLEEPING  = 4,
    ZOMBIE    = 5,
    DEAD      = 6,
};

// Process flags
enum ProcessFlags : std::uint32_t {
    FLAG_NONE        = 0,
    FLAG_SYSTEM      = 1 << 0,  // System process
    FLAG_KERNEL      = 1 << 1,  // Kernel-level process
    FLAG_DAEMON      = 1 << 2,  // Background daemon
    FLAG_DEBUGGED    = 1 << 3,  // Being debugged
};

// Process structure
struct Process {
    std::uint32_t id;
    ProcessState state;
    std::uint32_t flags;

    // Address space
    paging::AddressSpace address_space;

    // Threads
    scheduler::Thread* threads[MAX_THREADS_PER_PROCESS];
    std::size_t thread_count;
    scheduler::Thread* main_thread;

    // Process hierarchy
    std::uint32_t parent_id;
    std::uint32_t child_ids[MAX_PROCESSES];
    std::size_t child_count;

    // File descriptors
    int file_descriptors[32];
    std::size_t fd_count;

    // IPC
    ipc::Port* message_port;

    // Exit status
    int exit_code;
    bool exited;

    // Working directory
    char working_directory[256];

    // Environment
    char* environment[MAX_ARGUMENTS];
    std::size_t environment_count;

    // Linked list
    Process* next;
    Process* prev;
};

// Process creation info
struct CreateInfo {
    const char* name;
    void (*entry)(void*);
    void* arg;
    std::uint32_t flags;
    std::uint8_t priority;
};

// Process statistics
struct ProcessStats {
    std::uint32_t total_processes;
    std::uint32_t running_processes;
    std::uint32_t sleeping_processes;
    std::uint32_t zombie_processes;
    std::uint32_t total_threads;
};

// Initialize process management
bool initialize();

// Create a new process
Process* create(const CreateInfo& info);

// Create a kernel process (runs in kernel space)
Process* create_kernel(void (*entry)(void*), void* arg,
                         std::uint32_t flags = FLAG_SYSTEM);

// Destroy a process
void destroy(Process* process);

// Exit the current process
[[noreturn]] void exit(int code);

// Wait for a child process to exit
int wait(std::uint32_t process_id, int* out_code);

// Get a process by ID
Process* get_by_id(std::uint32_t id) noexcept;

// Get the current process
Process* current() noexcept;

// Fork the current process
Process* fork();

// Execute a program (from initrd)
int exec(Process* process, const char* path,
           const char* const argv[]);

// Send a signal to a process
int send_signal(std::uint32_t process_id, int signal);

// Process statistics
ProcessStats stats() noexcept;

// List all processes
std::size_t list(Process** out, std::size_t max) noexcept;

// Get process name
const char* name(const Process* process) noexcept;

// Set process name
void set_name(Process* process, const char* name);

// Change working directory
int chdir(Process* process, const char* path);

// Get working directory
const char* getcwd(const Process* process) noexcept;

// Add a file descriptor
int add_fd(Process* process, int fd);

// Remove a file descriptor
int remove_fd(Process* process, int fd);

// Register an exit handler
using ExitHandler = void (*)(Process*);
void set_exit_handler(ExitHandler handler);

} // namespace kernel::process