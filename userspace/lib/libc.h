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

#ifndef _NEBULA_LIBC_H
#define _NEBULA_LIBC_H

#include "syscalls.h"

#ifdef __cplusplus
extern "C" {
#endif

// ---- String functions ----
unsigned int strlen(const char* s);
char* strcpy(char* dest, const char* src);
char* strncpy(char* dest, const char* src, unsigned int n);
char* strcat(char* dest, const char* src);
int strcmp(const char* a, const char* b);
int strncmp(const char* a, const char* b, unsigned int n);
char* strchr(const char* s, int c);
char* strrchr(const char* s, int c);
unsigned int strspn(const char* s, const char* accept);
unsigned int strcspn(const char* s, const char* reject);
char* strpbrk(const char* s, const char* accept);
char* strstr(const char* haystack, const char* needle);
char* strtok(char* s, const char* delim);
void* memset(void* s, int c, unsigned int n);
void* memcpy(void* dest, const void* src, unsigned int n);
void* memmove(void* dest, const void* src, unsigned int n);
int memcmp(const void* a, const void* b, unsigned int n);
char* strdup(const char* s);
char* strdupn(const char* s, unsigned int n);

// ---- stdio ----
int puts(const char* s);
int fputs(const char* s, int fd);
int printf(const char* fmt, ...);
int sprintf(char* out, const char* fmt, ...);
int snprintf(char* out, unsigned int size, const char* fmt, ...);
int putchar(int c);
int fputc(int c, int fd);
int fputs_line(const char* s, int fd);
int getchar(void);
char* gets(char* buf, unsigned int n);
int fflush(int fd);

// ---- stdlib ----
int atoi(const char* s);
long atol(const char* s);
long long atoll(const char* s);
void* malloc(unsigned int size);
void free(void* ptr);
void* calloc(unsigned int count, unsigned int size);
void* realloc(void* ptr, unsigned int size);
void abort(void);
void exit(int status);
int abs(int n);
long labs(long n);

// ---- unistd ----
int open(const char* path, int flags);
int close(int fd);
int read(int fd, void* buf, unsigned int count);
int write(int fd, const void* buf, unsigned int count);
int lseek(int fd, int offset, int whence);
int fstat(int fd, void* stat);

// ---- framebuffer device ----
typedef struct {
    unsigned int buffer;
    unsigned int width;
    unsigned int height;
    unsigned int pitch;
    unsigned char bpp;
    unsigned char type;
    unsigned char red_position;
    unsigned char red_mask_size;
    unsigned char green_position;
    unsigned char green_mask_size;
    unsigned char blue_position;
    unsigned char blue_mask_size;
} framebuffer_info_t;

int open_framebuffer(framebuffer_info_t* info);
void close_framebuffer(int fd);
unsigned int fb_pixel(unsigned int x, unsigned int y);
void fb_put_pixel(unsigned int x, unsigned int y, unsigned int color);
void fb_fill_rect(unsigned int x, unsigned int y, unsigned int w, unsigned int h, unsigned int color);
void fb_present(void);
unsigned int fb_width(void);
unsigned int fb_height(void);

// ---- time ----
unsigned long long time_seconds(void);
unsigned int time_ticks(void);

// ---- misc ----
void* memset_bytes(void* s, int c, unsigned int n);
int isspace(int c);
int isdigit(int c);
int isalpha(int c);
int isalnum(int c);
int toupper(int c);
int tolower(int c);

#ifdef __cplusplus
}
#endif

#endif