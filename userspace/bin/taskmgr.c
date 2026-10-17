// Process/task manager application for NebulaOS.
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
// See LICENCE for the full license text.

#include "syscalls.h"
#include "libc.h"

// Mirrors kernel::syscall::SysInfo
struct sysinfo_t {
    unsigned int total_memory_mb;
    unsigned int free_memory_mb;
    unsigned int used_memory_mb;
    unsigned int uptime_seconds;
    unsigned int cpu_count;
    unsigned int cpu_frequency_mhz;
    char cpu_vendor[13];
    char cpu_model[49];
    char kernel_version[64];
    char build_date[32];
    char build_sha[32];
};

// Mirrors kernel::syscall::ProcessInfo
struct process_info_t {
    unsigned int id;
    unsigned int state;
    unsigned int priority;
    unsigned int total_time_ms;
    char name[32];
};

static const char* state_name(unsigned int state) {
    switch (state) {
        case 0: return "CREATED";
        case 1: return "READY";
        case 2: return "RUNNING";
        case 3: return "BLOCKED";
        case 4: return "SLEEPING";
        case 5: return "ZOMBIE";
        case 6: return "DEAD";
        default: return "UNKNOWN";
    }
}

static void print_dec(unsigned int val) {
    char buf[16];
    int i = 15;
    buf[i] = '\0';
    if (val == 0) {
        buf[--i] = '0';
    } else {
        while (val > 0) {
            buf[--i] = '0' + (val % 10);
            val /= 10;
        }
    }
    while (buf[i] != '\0') {
        write(1, &buf[i], 1);
        i++;
    }
}

static void print_str(const char* s) {
    while (*s) {
        write(1, s, 1);
        s++;
    }
}

int main(int argc, char** argv) {
    (void)argc;
    (void)argv;

    print_str("=== NebulaOS Task Manager ===\n\n");

    // System info
    struct sysinfo_t si;
    if (syscall_sysinfo(&si) == 0) {
        print_str("UPTIME: ");
        print_dec(si.uptime_seconds);
        print_str(" seconds\n");

        print_str("MEMORY: ");
        print_dec(si.used_memory_mb);
        print_str(" MB used / ");
        print_dec(si.total_memory_mb);
        print_str(" MB total (");
        print_dec(si.free_memory_mb);
        print_str(" MB free)\n");

        print_str("KERNEL: ");
        print_str(si.kernel_version);
        print_str("\n");
        print_str("BUILD: ");
        print_str(si.build_date);
        print_str(" sha=");
        print_str(si.build_sha);
        print_str("\n\n");
    } else {
        print_str("Could not read system info.\n\n");
    }

    // Process list
    print_str("PID  STATE      PRIORITY  NAME\n");
    print_str("---  -----      --------  ----\n");

    struct process_info_t plist[64];
    unsigned int count = syscall_process_list(plist, 64);
    for (unsigned int i = 0; i < count; ++i) {
        print_dec(plist[i].id);
        print_str("   ");
        print_str(state_name(plist[i].state));
        // Pad state name
        unsigned int slen = 0;
        while (state_name(plist[i].state)[slen]) slen++;
        for (unsigned int pad = slen; pad < 10; ++pad) {
            write(1, " ", 1);
        }
        print_dec(plist[i].priority);
        print_str("        ");
        print_str(plist[i].name);
        write(1, "\n", 1);
    }

    if (count == 0) {
        print_str("No processes found.\n");
    }

    print_str("\nTask manager complete.\n");
    return 0;
}
