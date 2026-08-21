// NebulaOS - Main Kernel Header
// ===============================
//
// Core definitions and includes for the NebulaOS kernel

#ifndef NEBULAOS_NEBULA_H
#define NEBULAOS_NEBULA_H

#include "stdint.h"

// Kernel version
#define NEBULAOS_VERSION "0.0.1"
#define NEBULAOS_NAME "NebulaOS"

// Architecture detection
#if defined(__i386__) || defined(_M_IX86)
#define NEBULAOS_ARCH_X86
#elif defined(__x86_64__) || defined(_M_X64)
#define NEBULAOS_ARCH_X86_64
#else
#error "Unsupported architecture"
#endif

// Kernel mode detection
#ifdef NEBULAOS_ARCH_X86
#define KERNEL_MODE 32
#else
#define KERNEL_MODE 64
#endif

// Compiler attributes
#define PACKED __attribute__((packed))
#define ALIGNED(n) __attribute__((aligned(n)))
#define NORETURN __attribute__((noreturn))
#define USED __attribute__((used))
#define WEAK __attribute__((weak))

// Section attributes
#define SECTION(code) __attribute__((section(".text." #code)))
#define SECTION_DATA(data) __attribute__((section(".data." #data)))

// Barrier macros for memory ordering
#define barrier() __asm__ __volatile__("": : : "memory")

// Likely/unlikely hints for branches
#define likely(x) __builtin_expect(!!(x), 1)
#define unlikely(x) __builtin_expect(!!(x), 0)

// Static assertion
#define STATIC_ASSERT(cond, msg) _Static_assert(cond, msg)

// Min and max macros
#define MIN(a, b) ((a) < (b) ? (a) : (b))
#define MAX(a, b) ((a) > (b) ? (a) : (b))

// Container_of macro
#define container_of(ptr, type, member) \
    ((type *)((char *)(ptr) - offsetof(type, member)))

// Bit manipulation
#define BIT(n) (1UL << (n))
#define BIT64(n) (1ULL << (n))

// Check if a number is a power of 2
#define IS_POWER_OF_2(x) ((x) != 0 && ((x) & ((x) - 1)) == 0)

// Align up/down
#define ALIGN_UP(x, a) (((x) + (a) - 1) & ~((a) - 1))
#define ALIGN_DOWN(x, a) ((x) & ~((a) - 1))

// Endianness conversion
#define bswap16(x) (__builtin_bswap16(x))
#define bswap32(x) (__builtin_bswap32(x))
#define bswap64(x) (__builtin_bswap64(x))

// Stringify macro
#define STRINGIFY(x) #x
#define TOSTRING(x) STRINGIFY(x)

// Offsetof macro (for compilers that don't have it)
#ifndef offsetof
#define offsetof(type, member) ((size_t) & ((type *)0)->member)
#endif

// Error codes
#define NEBULAOS_SUCCESS 0
#define NEBULAOS_ERROR -1
#define NEBULAOS_ENOMEM -2
#define NEBULAOS_EINVAL -3
#define NEBULAOS_ENOSYS -4

// Kernel panic function
void NORETURN kernel_panic(const char* message);

// Kernel logging levels
#define KERN_EMERG   0  // System is unusable
#define KERN_ALERT   1  // Action must be taken immediately
#define KERN_CRIT    2  // Critical conditions
#define KERN_ERR     3  // Error conditions
#define KERN_WARNING 4  // Warning conditions
#define KERN_NOTICE  5  // Normal but significant condition
#define KERN_INFO    6  // Informational
#define KERN_DEBUG   7  // Debug-level messages

#endif // NEBULAOS_NEBULA_H
