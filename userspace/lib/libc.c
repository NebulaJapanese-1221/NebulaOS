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

#include "libc.h"
#include "syscalls.h"

typedef int bool;
#define true 1
#define false 0

#define HEAP_START 0x100000
#define HEAP_MAX 0x1000000

static uint32_t heap_ptr = HEAP_START;
static uint32_t heap_end = HEAP_MAX;

static void* sbrk(int32_t increment) {
    if (heap_ptr + increment > heap_end) {
        return (void*)0xFFFFFFFF;
    }
    void* old = (void*)heap_ptr;
    heap_ptr += increment;
    return old;
}

// ---- String functions ----
unsigned int strlen(const char* s) {
    unsigned int len = 0;
    while (s[len]) len++;
    return len;
}

char* strcpy(char* dest, const char* src) {
    char* d = dest;
    while ((*d++ = *src++));
    return dest;
}

char* strncpy(char* dest, const char* src, unsigned int n) {
    char* d = dest;
    while (n-- && (*d++ = *src++));
    while (n--) *d++ = '\0';
    return dest;
}

char* strcat(char* dest, const char* src) {
    char* d = dest + strlen(dest);
    while ((*d++ = *src++));
    return dest;
}

int strcmp(const char* a, const char* b) {
    while (*a && *a == *b) {
        a++; b++;
    }
    return *(unsigned char*)a - *(unsigned char*)b;
}

int strncmp(const char* a, const char* b, unsigned int n) {
    while (n-- && *a && *a == *b) {
        a++; b++;
    }
    if (n == (unsigned int)-1) return 0;
    return *(unsigned char*)a - *(unsigned char*)b;
}

char* strchr(const char* s, int c) {
    while (*s) {
        if (*s == (char)c) return (char*)s;
        s++;
    }
    if (c == '\0') return (char*)s;
    return 0;
}

char* strrchr(const char* s, int c) {
    const char* last = 0;
    while (*s) {
        if (*s == (char)c) last = s;
        s++;
    }
    if (c == '\0') return (char*)s;
    return (char*)last;
}

unsigned int strspn(const char* s, const char* accept) {
    unsigned int count = 0;
    while (*s) {
        const char* a = accept;
        bool found = false;
        while (*a) {
            if (*s == *a) { found = true; break; }
            a++;
        }
        if (!found) break;
        count++; s++;
    }
    return count;
}

unsigned int strcspn(const char* s, const char* reject) {
    unsigned int count = 0;
    while (*s) {
        const char* r = reject;
        while (*r) {
            if (*s == *r) return count;
            r++;
        }
        count++; s++;
    }
    return count;
}

char* strpbrk(const char* s, const char* accept) {
    while (*s) {
        const char* a = accept;
        while (*a) {
            if (*s == *a) return (char*)s;
            a++;
        }
        s++;
    }
    return 0;
}

char* strstr(const char* haystack, const char* needle) {
    if (!*needle) return (char*)haystack;
    for (; *haystack; haystack++) {
        const char* h = haystack;
        const char* n = needle;
        while (*h && *n && *h == *n) {
            h++; n++;
        }
        if (!*n) return (char*)haystack;
    }
    return 0;
}

char* strtok(char* s, const char* delim) {
    static char* last = 0;
    if (s) last = s;
    if (!last) return 0;
    s = last;
    s += strspn(s, delim);
    if (!*s) return last = 0;
    char* token = s;
    s += strcspn(s, delim);
    if (*s) *s++ = '\0';
    last = s;
    return token;
}

void* memset(void* s, int c, unsigned int n) {
    unsigned char* p = (unsigned char*)s;
    while (n--) *p++ = (unsigned char)c;
    return s;
}

void* memcpy(void* dest, const void* src, unsigned int n) {
    unsigned char* d = (unsigned char*)dest;
    const unsigned char* s = (const unsigned char*)src;
    while (n--) *d++ = *s++;
    return dest;
}

void* memmove(void* dest, const void* src, unsigned int n) {
    unsigned char* d = (unsigned char*)dest;
    const unsigned char* s = (const unsigned char*)src;
    if (d < s) {
        while (n--) *d++ = *s++;
    } else {
        d += n; s += n;
        while (n--) *--d = *--s;
    }
    return dest;
}

int memcmp(const void* a, const void* b, unsigned int n) {
    const unsigned char* p1 = (const unsigned char*)a;
    const unsigned char* p2 = (const unsigned char*)b;
    while (n--) {
        if (*p1 != *p2) return *p1 - *p2;
        p1++; p2++;
    }
    return 0;
}

char* strdup(const char* s) {
    unsigned int len = strlen(s) + 1;
    char* copy = (char*)malloc(len);
    if (copy) memcpy(copy, s, len);
    return copy;
}

char* strdupn(const char* s, unsigned int n) {
    unsigned int len = strlen(s);
    if (len > n) len = n;
    char* copy = (char*)malloc(len + 1);
    if (copy) {
        memcpy(copy, s, len);
        copy[len] = '\0';
    }
    return copy;
}

// ---- stdio ----
static int stdout_fd = 1;

int puts(const char* s) {
    unsigned int len = strlen(s);
    write(stdout_fd, s, len);
    write(stdout_fd, "\n", 1);
    return len + 1;
}

int fputs(const char* s, int fd) {
    unsigned int len = strlen(s);
    return write(fd, s, len);
}

int putchar(int c) {
    char ch = (char)c;
    return write(stdout_fd, &ch, 1) == 1 ? c : -1;
}

int fputc(int c, int fd) {
    char ch = (char)c;
    return write(fd, &ch, 1) == 1 ? c : -1;
}

int fputs_line(const char* s, int fd) {
    int r = fputs(s, fd);
    if (r >= 0) r += write(fd, "\n", 1);
    return r;
}

int getchar(void) {
    char c;
    int r = read(0, &c, 1);
    return r == 1 ? (unsigned char)c : -1;
}

char* gets(char* buf, unsigned int n) {
    unsigned int i = 0;
    while (i < n - 1) {
        int c = getchar();
        if (c == -1 || c == '\n') break;
        if (c == '\b' || c == 0x7F) {
            if (i > 0) {
                i--;
                putchar('\b');
                putchar(' ');
                putchar('\b');
            }
        } else {
            buf[i++] = (char)c;
            putchar(c);
        }
    }
    buf[i] = '\0';
    putchar('\n');
    return i > 0 ? buf : 0;
}

int fflush(int fd) {
    (void)fd;
    return 0;
}

static void print_number(unsigned int val, int base, int width, char pad) {
    char buf[32];
    int i = 0;
    if (val == 0) {
        buf[i++] = '0';
    } else {
        while (val > 0) {
            int digit = val % base;
            buf[i++] = (digit < 10) ? '0' + digit : 'A' + digit - 10;
            val /= base;
        }
    }
    while (i < width) {
        putchar(pad);
        width--;
    }
    while (i--) putchar(buf[i]);
}

int printf(const char* fmt, ...) {
    va_list args;
    va_start(args, fmt);
    int count = 0;
    while (*fmt) {
        if (*fmt == '%') {
            fmt++;
            int width = 0;
            char pad = ' ';
            if (*fmt == '0') { pad = '0'; fmt++; }
            while (*fmt >= '0' && *fmt <= '9') {
                width = width * 10 + (*fmt - '0');
                fmt++;
            }
            switch (*fmt) {
                case 'd': {
                    int val = va_arg(args, int);
                    if (val < 0) { putchar('-'); count++; val = -val; }
                    print_number(val, 10, width, pad);
                    count += width > 0 ? width : 1;
                    break;
                }
                case 'u':
                    print_number(va_arg(args, unsigned int), 10, width, pad);
                    count += width > 0 ? width : 1;
                    break;
                case 'x':
                case 'X':
                    print_number(va_arg(args, unsigned int), 16, width, pad);
                    count += width > 0 ? width : 1;
                    break;
                case 'p':
                    putchar('0'); putchar('x'); count += 2;
                    print_number(va_arg(args, unsigned int), 16, 8, '0');
                    count += 8;
                    break;
                case 's': {
                    const char* s = va_arg(args, const char*);
                    if (!s) s = "(null)";
                    while (*s) { putchar(*s++); count++; }
                    break;
                }
                case 'c':
                    putchar(va_arg(args, int));
                    count++;
                    break;
                case '%':
                    putchar('%');
                    count++;
                    break;
                default:
                    putchar('%');
                    putchar(*fmt);
                    count += 2;
                    break;
            }
        } else {
            putchar(*fmt);
            count++;
        }
        fmt++;
    }
    va_end(args);
    return count;
}

int sprintf(char* out, const char* fmt, ...) {
    va_list args;
    va_start(args, fmt);
    char* start = out;
    while (*fmt) {
        if (*fmt == '%') {
            fmt++;
            int width = 0;
            char pad = ' ';
            if (*fmt == '0') { pad = '0'; fmt++; }
            while (*fmt >= '0' && *fmt <= '9') {
                width = width * 10 + (*fmt - '0');
                fmt++;
            }
            switch (*fmt) {
                case 'd': {
                    int val = va_arg(args, int);
                    if (val < 0) { *out++ = '-'; val = -val; }
                    char buf[32];
                    int i = 0;
                    if (val == 0) buf[i++] = '0';
                    else while (val > 0) { buf[i++] = '0' + val % 10; val /= 10; }
                    while (i < width) { *out++ = pad; width--; }
                    while (i--) *out++ = buf[i];
                    break;
                }
                case 'u': {
                    unsigned int val = va_arg(args, unsigned int);
                    char buf[32];
                    int i = 0;
                    if (val == 0) buf[i++] = '0';
                    else while (val > 0) { buf[i++] = '0' + val % 10; val /= 10; }
                    while (i < width) { *out++ = pad; width--; }
                    while (i--) *out++ = buf[i];
                    break;
                }
                case 'x':
                case 'X': {
                    unsigned int val = va_arg(args, unsigned int);
                    char buf[32];
                    int i = 0;
                    if (val == 0) buf[i++] = '0';
                    else while (val > 0) {
                        int d = val % 16;
                        buf[i++] = (d < 10) ? '0' + d : 'A' + d - 10;
                        val /= 16;
                    }
                    while (i < width) { *out++ = pad; width--; }
                    while (i--) *out++ = buf[i];
                    break;
                }
                case 's': {
                    const char* s = va_arg(args, const char*);
                    if (!s) s = "(null)";
                    while (*s) *out++ = *s++;
                    break;
                }
                case 'c':
                    *out++ = (char)va_arg(args, int);
                    break;
                case '%':
                    *out++ = '%';
                    break;
                default:
                    *out++ = '%'; *out++ = *fmt;
                    break;
            }
        } else {
            *out++ = *fmt;
        }
        fmt++;
    }
    *out = '\0';
    va_end(args);
    return out - start;
}

int snprintf(char* out, unsigned int size, const char* fmt, ...) {
    va_list args;
    va_start(args, fmt);
    char* start = out;
    char* end = out + size - 1;
    while (*fmt && out < end) {
        if (*fmt == '%') {
            fmt++;
            int width = 0;
            char pad = ' ';
            if (*fmt == '0') { pad = '0'; fmt++; }
            while (*fmt >= '0' && *fmt <= '9') {
                width = width * 10 + (*fmt - '0');
                fmt++;
            }
            switch (*fmt) {
                case 'd': {
                    int val = va_arg(args, int);
                    if (val < 0) { if (out < end) *out++ = '-'; val = -val; }
                    char buf[32];
                    int i = 0;
                    if (val == 0) buf[i++] = '0';
                    else while (val > 0) { buf[i++] = '0' + val % 10; val /= 10; }
                    while (i < width && out < end) { *out++ = pad; width--; }
                    while (i-- && out < end) *out++ = buf[i];
                    break;
                }
                case 'u': {
                    unsigned int val = va_arg(args, unsigned int);
                    char buf[32];
                    int i = 0;
                    if (val == 0) buf[i++] = '0';
                    else while (val > 0) { buf[i++] = '0' + val % 10; val /= 10; }
                    while (i < width && out < end) { *out++ = pad; width--; }
                    while (i-- && out < end) *out++ = buf[i];
                    break;
                }
                case 'x':
                case 'X': {
                    unsigned int val = va_arg(args, unsigned int);
                    char buf[32];
                    int i = 0;
                    if (val == 0) buf[i++] = '0';
                    else while (val > 0) {
                        int d = val % 16;
                        buf[i++] = (d < 10) ? '0' + d : 'A' + d - 10;
                        val /= 16;
                    }
                    while (i < width && out < end) { *out++ = pad; width--; }
                    while (i-- && out < end) *out++ = buf[i];
                    break;
                }
                case 's': {
                    const char* s = va_arg(args, const char*);
                    if (!s) s = "(null)";
                    while (*s && out < end) *out++ = *s++;
                    break;
                }
                case 'c':
                    if (out < end) *out++ = (char)va_arg(args, int);
                    break;
                case '%':
                    if (out < end) *out++ = '%';
                    break;
                default:
                    if (out < end) *out++ = '%';
                    if (out < end) *out++ = *fmt;
                    break;
            }
        } else {
            if (out < end) *out++ = *fmt;
        }
        fmt++;
    }
    *out = '\0';
    va_end(args);
    return out - start;
}

// ---- stdlib ----
int atoi(const char* s) {
    int sign = 1;
    int val = 0;
    while (*s == ' ' || *s == '\t' || *s == '\n') s++;
    if (*s == '-') { sign = -1; s++; }
    else if (*s == '+') s++;
    while (*s >= '0' && *s <= '9') {
        val = val * 10 + (*s - '0');
        s++;
    }
    return val * sign;
}

long atol(const char* s) {
    long sign = 1;
    long val = 0;
    while (*s == ' ' || *s == '\t' || *s == '\n') s++;
    if (*s == '-') { sign = -1; s++; }
    else if (*s == '+') s++;
    while (*s >= '0' && *s <= '9') {
        val = val * 10 + (*s - '0');
        s++;
    }
    return val * sign;
}

long long atoll(const char* s) {
    long long sign = 1;
    long long val = 0;
    while (*s == ' ' || *s == '\t' || *s == '\n') s++;
    if (*s == '-') { sign = -1; s++; }
    else if (*s == '+') s++;
    while (*s >= '0' && *s <= '9') {
        val = val * 10 + (*s - '0');
        s++;
    }
    return val * sign;
}

void* malloc(unsigned int size) {
    if (size == 0) return 0;
    size = (size + 3) & ~3;
    void* ptr = sbrk(size);
    if (ptr == (void*)0xFFFFFFFF) return 0;
    return ptr;
}

void free(void* ptr) {
    (void)ptr;
}

void* calloc(unsigned int count, unsigned int size) {
    unsigned int total = count * size;
    void* ptr = malloc(total);
    if (ptr) memset(ptr, 0, total);
    return ptr;
}

void* realloc(void* ptr, unsigned int size) {
    if (!ptr) return malloc(size);
    if (size == 0) { free(ptr); return 0; }
    void* new_ptr = malloc(size);
    if (!new_ptr) return 0;
    memcpy(new_ptr, ptr, size);
    free(ptr);
    return new_ptr;
}

void abort(void) {
    _exit(1);
    for (;;) asm volatile("hlt");
}

int abs(int n) {
    return n < 0 ? -n : n;
}

long labs(long n) {
    return n < 0 ? -n : n;
}

// ---- unistd ----
int open(const char* path, int flags) {
    (void)flags;
    return open_device(path, 0, 0);
}

int close(int fd) {
    (void)fd;
    return 0;
}

int lseek(int fd, int offset, int whence) {
    (void)fd; (void)offset; (void)whence;
    return 0;
}

int fstat(int fd, void* stat) {
    (void)fd; (void)stat;
    return 0;
}

// ---- framebuffer device ----
int open_framebuffer(framebuffer_info_t* info) {
    int fd = open("fb", 0);
    if (fd < 0) return -1;
    return get_framebuffer_info(fd, info);
}

void close_framebuffer(int fd) {
    close(fd);
}

unsigned int fb_pixel(unsigned int x, unsigned int y) {
    (void)x; (void)y;
    return 0;
}

void fb_put_pixel(unsigned int x, unsigned int y, unsigned int color) {
    (void)x; (void)y; (void)color;
}

void fb_fill_rect(unsigned int x, unsigned int y, unsigned int w, unsigned int h, unsigned int color) {
    (void)x; (void)y; (void)w; (void)h; (void)color;
}

void fb_present(void) {
}

unsigned int fb_width(void) {
    return 0;
}

unsigned int fb_height(void) {
    return 0;
}

// ---- time ----
unsigned long long time_seconds(void) {
    return get_time_seconds();
}

unsigned int time_ticks(void) {
    return get_ticks();
}

// ---- misc ----
void* memset_bytes(void* s, int c, unsigned int n) {
    return memset(s, c, n);
}

int isspace(int c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\v' || c == '\f';
}

int isdigit(int c) {
    return c >= '0' && c <= '9';
}

int isalpha(int c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}

int isalnum(int c) {
    return isdigit(c) || isalpha(c);
}

int toupper(int c) {
    return (c >= 'a' && c <= 'z') ? c - 'a' + 'A' : c;
}

int tolower(int c) {
    return (c >= 'A' && c <= 'Z') ? c - 'A' + 'a' : c;
}