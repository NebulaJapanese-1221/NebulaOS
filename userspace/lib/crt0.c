// Userspace C runtime for NebulaOS.
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

#include "syscalls.h"

extern int main(int argc, char** argv);
extern void _init(void);
extern void _fini(void);

// atexit handler table
#define ATEXIT_MAX 32
static void (*atexit_handlers[ATEXIT_MAX])(void);
static int atexit_count = 0;

int atexit(void (*func)(void)) {
    if (atexit_count >= ATEXIT_MAX) {
        return -1;
    }
    atexit_handlers[atexit_count++] = func;
    return 0;
}

void exit(int status) {
    // Run atexit handlers in reverse order.
    while (atexit_count > 0) {
        atexit_handlers[--atexit_count]();
    }
    // Run the fini section.
    _fini();
    _exit(status);
}

void _start_c(int argc, char** argv) {
    // Run the init section first.
    _init();

    int result = main(argc, argv);
    exit(result);
}

// Dummy init/fini (overridden by linker section markers if present).
void _init(void) {}
void _fini(void) {}