// NebulaOS - Standard Integer Types
// ====================================
//
// Defines standard integer types for the kernel
// Compatible with C99 stdint.h

#ifndef NEBULAOS_STDINT_H
#define NEBULAOS_STDINT_H

// Basic types
#ifndef NULL
#define NULL ((void*)0)
#endif

// Fixed-width integer types
typedef unsigned char       uint8_t;
typedef unsigned short      uint16_t;
typedef unsigned int        uint32_t;
typedef unsigned long long  uint64_t;

typedef signed char         int8_t;
typedef signed short        int16_t;
typedef signed int          int32_t;
typedef signed long long    int64_t;

// Architecture-specific types
#ifdef __x86_64__
#define ARCH_64BIT
#else
#define ARCH_32BIT
#endif

// Pointer-sized types
typedef unsigned long       size_t;
typedef signed long         ssize_t;

// Boolean type
#ifndef __cplusplus
typedef int bool;
#define true 1
#define false 0
#endif

// Min and max for types
#define UINT8_MAX  255
#define UINT16_MAX 65535
#define UINT32_MAX 4294967295U
#define UINT64_MAX 18446744073709551615ULL

#define INT8_MAX   127
#define INT16_MAX  32767
#define INT32_MAX  2147483647
#define INT64_MAX  9223372036854775807LL

#endif // NEBULAOS_STDINT_H
