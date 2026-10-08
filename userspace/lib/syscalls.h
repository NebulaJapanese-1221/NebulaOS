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

#include <cstdint>
#include <cstddef>

typedef uint32_t uint32_t;
typedef int int32_t;
typedef uint16_t uint16_t;
typedef int16_t int16_t;
typedef uint8_t uint8_t;
typedef int8_t int8_t;
typedef unsigned long size_t;
typedef long ssize_t;
typedef unsigned int mode_t;
typedef long off_t;

typedef __builtin_va_list va_list;
#define va_start(ap, last) __builtin_va_start(ap, last)
#define va_end(ap) __builtin_va_end(ap)
#define va_arg(ap, type) __builtin_va_arg(ap, type)

// Syscall numbers
#define SYS_WRITE           1
#define SYS_EXIT            60
#define SYS_GETPID          20
#define SYS_OPEN_DEVICE     200
#define SYS_GET_FB_INFO     201
#define SYS_GET_TIME        202
#define SYS_GET_TICKS       203

// New syscall numbers for POSIX compatibility
#define SYS_OPEN            210
#define SYS_CLOSE           211
#define SYS_READ            212
#define SYS_WRITE_NEW       213
#define SYS_LSEEK           214
#define SYS_FSTAT           215
#define SYS_STAT            216
#define SYS_LSTAT           217
#define SYS_ACCESS          218
#define SYS_DUP             219
#define SYS_DUP2            220
#define SYS_PIPE            221
#define SYS_ISATTY          222
#define SYS_TRUNCATE        223
#define SYS_FTRUNCATE       224
#define SYS_READLINK        225
#define SYS_SYMLINK         226
#define SYS_LINK            227
#define SYS_UNLINK          228
#define SYS_RMDIR           229
#define SYS_MKDIR           230
#define SYS_CHDIR           231
#define SYS_FCHDIR          232
#define SYS_GETCWD          233
#define SYS_CHROOT          234
#define SYS_FSYNC           235
#define SYS_FDATASYNC       236
#define SYS_GETPPID         237
#define SYS_NANOSLEEP       238
#define SYS_PRESENT         239

static inline uint32_t syscall0(uint32_t num) {
    uint32_t ret;
    asm volatile("int $0x80" : "=a"(ret) : "a"(num) : "memory");
    return ret;
}

static inline uint32_t syscall1(uint32_t num, uint32_t arg1) {
    uint32_t ret;
    asm volatile("int $0x80" : "=a"(ret) : "a"(num), "b"(arg1) : "memory");
    return ret;
}

static inline uint32_t syscall2(uint32_t num, uint32_t arg1, uint32_t arg2) {
    uint32_t ret;
    asm volatile("int $0x80" : "=a"(ret) : "a"(num), "b"(arg1), "c"(arg2) : "memory");
    return ret;
}

static inline uint32_t syscall3(uint32_t num, uint32_t arg1, uint32_t arg2, uint32_t arg3) {
    uint32_t ret;
    asm volatile("int $0x80" : "=a"(ret) : "a"(num), "b"(arg1), "c"(arg2), "d"(arg3) : "memory");
    return ret;
}

static inline uint32_t syscall4(uint32_t num, uint32_t arg1, uint32_t arg2, uint32_t arg3, uint32_t arg4) {
    uint32_t ret;
    asm volatile("int $0x80" : "=a"(ret) : "a"(num), "b"(arg1), "c"(arg2), "d"(arg3), "S"(arg4) : "memory");
    return ret;
}

static inline uint32_t syscall5(uint32_t num, uint32_t arg1, uint32_t arg2, uint32_t arg3, uint32_t arg4, uint32_t arg5) {
    uint32_t ret;
    asm volatile("int $0x80" : "=a"(ret) : "a"(num), "b"(arg1), "c"(arg2), "d"(arg3), "S"(arg4), "D"(arg5) : "memory");
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

// New syscall wrappers for POSIX compatibility
static inline int syscall_open(const char* path, int flags, mode_t mode) {
    return syscall3(SYS_OPEN, (uint32_t)path, (uint32_t)flags, (uint32_t)mode);
}

static inline int syscall_close(int fd) {
    return syscall1(SYS_CLOSE, (uint32_t)fd);
}

static inline ssize_t syscall_read(int fd, void* buf, size_t count) {
    return syscall3(SYS_READ, (uint32_t)fd, (uint32_t)buf, (uint32_t)count);
}

static inline ssize_t syscall_write(int fd, const void* buf, size_t count) {
    return syscall3(SYS_WRITE_NEW, (uint32_t)fd, (uint32_t)buf, (uint32_t)count);
}

static inline off_t syscall_lseek(int fd, off_t offset, int whence) {
    return syscall3(SYS_LSEEK, (uint32_t)fd, (uint32_t)offset, (uint32_t)whence);
}

static inline int syscall_fstat(int fd, void* stat) {
    return syscall2(SYS_FSTAT, (uint32_t)fd, (uint32_t)stat);
}

static inline int syscall_stat(const char* path, void* stat) {
    return syscall2(SYS_STAT, (uint32_t)path, (uint32_t)stat);
}

static inline int syscall_lstat(const char* path, void* stat) {
    return syscall2(SYS_LSTAT, (uint32_t)path, (uint32_t)stat);
}

static inline int syscall_access(const char* path, int mode) {
    return syscall2(SYS_ACCESS, (uint32_t)path, (uint32_t)mode);
}

static inline int syscall_dup(int fd) {
    return syscall1(SYS_DUP, (uint32_t)fd);
}

static inline int syscall_dup2(int oldfd, int newfd) {
    return syscall2(SYS_DUP2, (uint32_t)oldfd, (uint32_t)newfd);
}

static inline int syscall_pipe(int fds[2]) {
    return syscall1(SYS_PIPE, (uint32_t)fds);
}

static inline int syscall_isatty(int fd) {
    return syscall1(SYS_ISATTY, (uint32_t)fd);
}

static inline int syscall_truncate(const char* path, off_t length) {
    return syscall2(SYS_TRUNCATE, (uint32_t)path, (uint32_t)length);
}

static inline int syscall_ftruncate(int fd, off_t length) {
    return syscall2(SYS_FTRUNCATE, (uint32_t)fd, (uint32_t)length);
}

static inline ssize_t syscall_readlink(const char* path, char* buf, size_t bufsiz) {
    return syscall3(SYS_READLINK, (uint32_t)path, (uint32_t)buf, (uint32_t)bufsiz);
}

static inline int syscall_symlink(const char* target, const char* linkpath) {
    return syscall2(SYS_SYMLINK, (uint32_t)target, (uint32_t)linkpath);
}

static inline int syscall_link(const char* oldpath, const char* newpath) {
    return syscall2(SYS_LINK, (uint32_t)oldpath, (uint32_t)newpath);
}

static inline int syscall_unlink(const char* path) {
    return syscall1(SYS_UNLINK, (uint32_t)path);
}

static inline int syscall_rmdir(const char* path) {
    return syscall1(SYS_RMDIR, (uint32_t)path);
}

static inline int syscall_mkdir(const char* path, mode_t mode) {
    return syscall2(SYS_MKDIR, (uint32_t)path, (uint32_t)mode);
}

static inline int syscall_chdir(const char* path) {
    return syscall1(SYS_CHDIR, (uint32_t)path);
}

static inline int syscall_fchdir(int fd) {
    return syscall1(SYS_FCHDIR, (uint32_t)fd);
}

static inline char* syscall_getcwd(char* buf, size_t size) {
    return (char*)syscall2(SYS_GETCWD, (uint32_t)buf, (uint32_t)size);
}

static inline int syscall_chroot(const char* path) {
    return syscall1(SYS_CHROOT, (uint32_t)path);
}

static inline int syscall_fsync(int fd) {
    return syscall1(SYS_FSYNC, (uint32_t)fd);
}

static inline int syscall_fdatasync(int fd) {
    return syscall1(SYS_FDATASYNC, (uint32_t)fd);
}

static inline int syscall_getppid(void) {
    return syscall0(SYS_GETPPID);
}

static inline int syscall_nanosleep(unsigned int ticks) {
    return syscall1(SYS_NANOSLEEP, ticks);
}

static inline void syscall_present(void) {
    syscall0(SYS_PRESENT);
}

static inline unsigned int syscall_time(void) {
    return syscall1(SYS_GET_TIME, 0);
}

static inline unsigned int syscall_ticks(void) {
    return syscall1(SYS_GET_TICKS, 0);
}

static inline void syscall_exit(int status) {
    (void)status;
    syscall1(SYS_EXIT, 0);
}

#endif
