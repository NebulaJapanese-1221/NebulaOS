// NebulaOS - String Library Header
// ================================
//
// String manipulation functions

#ifndef NEBULAOS_LIB_STRING_H
#define NEBULAOS_LIB_STRING_H

#include "stdint.h"
#include <stdarg.h>

#ifdef __cplusplus
extern "C" {
#endif

// Memory manipulation
void* memset(void* dest, int c, size_t n);
void* memcpy(void* dest, const void* src, size_t n);
void* memmove(void* dest, const void* src, size_t n);
int memcmp(const void* s1, const void* s2, size_t n);

// String manipulation
size_t strlen(const char* str);
char* strcpy(char* dest, const char* src);
char* strncpy(char* dest, const char* src, size_t n);
char* strcat(char* dest, const char* src);
char* strncat(char* dest, const char* src, size_t n);
int strcmp(const char* s1, const char* s2);
int strncmp(const char* s1, const char* s2, size_t n);

// Character search
char* strchr(char* str, int c);
char* strrchr(char* str, int c);

// String duplication
char* strdup(const char* str);

// Formatted output
int snprintf(char* buffer, size_t size, const char* format, ...);
int vsnprintf(char* buffer, size_t size, const char* format, va_list args);

#ifdef __cplusplus
}
#endif

#endif // NEBULAOS_LIB_STRING_H
