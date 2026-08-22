#ifndef NEBULAOS_STDIO_H
#define NEBULAOS_STDIO_H

#include "stdint.h"
#include <stdarg.h>

typedef int FILE;

#define stdout ((FILE*)1)
#define stdin ((FILE*)2)
#define stderr ((FILE*)3)

#define EOF (-1)

int printf(const char* format, ...);
int snprintf(char* buffer, size_t size, const char* format, ...);
int sprintf(char* buffer, const char* format, ...);
int vsnprintf(char* buffer, size_t size, const char* format, va_list args);

int putchar(int c);
int puts(const char* str);

#endif