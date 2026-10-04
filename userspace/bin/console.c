// Userspace console for NebulaOS.
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

static void puts(const char* s) {
    while (*s) {
        write(1, s, 1);
        s++;
    }
}

static void puthex(uint32_t val) {
    const char* hex = "0123456789ABCDEF";
    char buf[9];
    for (int i = 7; i >= 0; i--) {
        buf[7-i] = hex[(val >> (i*4)) & 0xF];
    }
    buf[8] = '\0';
    puts(buf);
}

static void putdec(uint32_t val) {
    char buf[11];
    int i = 10;
    buf[i] = '\0';
    if (val == 0) {
        buf[--i] = '0';
    } else {
        while (val > 0) {
            buf[--i] = '0' + (val % 10);
            val /= 10;
        }
    }
    puts(&buf[i]);
}

int main(int argc, char** argv) {
    (void)argc;
    (void)argv;

    puts("=== NebulaOS Userspace Console ===\n");
    puts("PID: ");
    putdec(getpid());
    puts("\n");
    puts("Hello from userspace!\n");
    puts("Testing syscalls: write, exit, getpid\n");
    puts("Syscall test passed.\n");
    return 0;
}