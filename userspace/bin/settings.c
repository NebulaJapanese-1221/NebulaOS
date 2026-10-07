// System settings application for NebulaOS.
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
#include "libc.h"

static void print(const char* s) {
    while (*s) {
        write(1, s, 1);
        s++;
    }
}

static void print_line(const char* s) {
    print(s);
    write(1, "\n", 1);
}

static void print_number(unsigned int val) {
    char buf[16];
    int i = 14;
    buf[15] = '\0';
    if (val == 0) {
        buf[--i] = '0';
    } else {
        while (val > 0) {
            buf[--i] = '0' + (val % 10);
            val /= 10;
        }
    }
    print(&buf[i]);
}

static void print_hex(unsigned int val) {
    const char* hex = "0123456789ABCDEF";
    char buf[11];
    for (int i = 7; i >= 0; i--) {
        buf[7 - i] = hex[(val >> (i * 4)) & 0xF];
    }
    buf[8] = '\0';
    print(buf);
}

int main(int argc, char** argv) {
    (void)argc;
    (void)argv;

    print_line("=== NEBULAOS SYSTEM SETTINGS ===");
    print_line("");

    print_line("KERNEL INFORMATION:");
    print("  VERSION: ");
    print_line("NEBULAOS 0.0.1");
    print("  ARCHITECTURE: ");
    print_line("32 BIT X86");
    print("  LICENSE: ");
    print_line("GNU GPL VERSION 3");
    print_line("");

    print_line("SYSTEM STATUS:");
    print("  UPTIME: ");
    print_number(time_seconds());
    print_line(" SECONDS");
    print("  TICKS: ");
    print_number(time_ticks());
    print_line("");
    print("  PID: ");
    print_number(getpid());
    print_line("");
    print_line("");

    print_line("MEMORY INFORMATION:");
    print("  HEAP START: 0x");
    print_hex(0x100000);
    print_line("");
    print("  HEAP END: 0x");
    print_hex(0x1000000);
    print_line("");
    print_line("");

    print_line("FRAMEBUFFER DEVICE:");
    framebuffer_info_t fb_info;
    int fb_fd = open_framebuffer(&fb_info);
    if (fb_fd >= 0) {
        print("  FD: ");
        print_number(fb_fd);
        print_line("");
        print("  WIDTH: ");
        print_number(fb_info.width);
        print_line("");
        print("  HEIGHT: ");
        print_number(fb_info.height);
        print_line("");
        print("  BPP: ");
        print_number(fb_info.bpp);
        print_line("");
        close_framebuffer(fb_fd);
    } else {
        print_line("  FRAMEBUFFER NOT AVAILABLE");
    }
    print_line("");

    print_line("SERIAL DEVICE:");
    int ser_fd = open("ser", 0);
    if (ser_fd >= 0) {
        print("  FD: ");
        print_number(ser_fd);
        print_line("");
        close(ser_fd);
    } else {
        print_line("  SERIAL NOT AVAILABLE");
    }
    print_line("");

    print_line("SYSTEM SETTINGS CONFIGURATION:");
    print_line("  BRIGHTNESS: 80%");
    print_line("  VOLUME: 50%");
    print_line("  POWER MODE: BALANCED");
    print_line("  AUTO-REBOOT: DISABLED");
    print_line("");

    print_line("SETTINGS APPLICATION COMPLETE.");
    return 0;
}