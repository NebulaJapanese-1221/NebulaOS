// Userspace C library for NebulaOS.
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

#ifndef _NEBULA_SYSCALLS_H
#define _NEBULA_SYSCALLS_H

typedef unsigned int uint32_t;
typedef int int32_t;
typedef unsigned short uint16_t;
typedef short int16_t;
typedef unsigned char uint8_t;
typedef char int8_t;
typedef unsigned long size_t;

typedef __builtin_va_list va_list;
#define va_start(ap, last) __builtin_va_start(ap, last)
#define va_end(ap) __builtin_va_end(ap)
#define va_arg(ap, type) __builtin_va_arg(ap, type)

#define SYS_WRITE           1
#define SYS_EXIT            60
#define SYS_GETPID          20
#define SYS_OPEN_DEVICE     200
#define SYS_GET_FB_INFO     201
#define SYS_GET_TIME        202
#define SYS_GET_TICKS       203

static inline uint32_t syscall1(uint32_t num, uint32_t arg1) {
    uint32_t ret;
    asm volatile("int $0x80" : "=a"(ret) : "a"(num), "b"(arg1) : "memory");
    return ret;
}

static inline uint32_t syscall3(uint32_t num, uint32_t arg1, uint32_t arg2, uint32_t arg3) {
    uint32_t ret;
    asm volatile("int $0x80" : "=a"(ret) : "a"(num), "b"(arg1), "c"(arg2), "d"(arg3) : "memory");
    return ret;
}

static inline int write(int fd, const void* buf, uint32_t count) {
    return syscall3(SYS_WRITE, fd, (uint32_t)buf, count);
}

static inline void _exit(int status) {
    (void)status;
    syscall1(SYS_EXIT, 0);
    for (;;) asm volatile("hlt");
}

static inline int getpid(void) {
    return syscall1(SYS_GETPID, 0);
}

static inline int open_device(const char* name, uint32_t buffer, uint32_t length) {
    return syscall3(SYS_OPEN_DEVICE, (uint32_t)name, buffer, length);
}

static inline int get_framebuffer_info(int fd, void* info) {
    return syscall3(SYS_GET_FB_INFO, fd, (uint32_t)info, 0);
}

static inline unsigned int get_time_seconds(void) {
    return syscall1(SYS_GET_TIME, 0);
}

static inline unsigned int get_ticks(void) {
    return syscall1(SYS_GET_TICKS, 0);
}

static inline int read(int fd, void* buf, uint32_t count) {
    return syscall3(SYS_WRITE, fd, (uint32_t)buf, count);
}

#endif