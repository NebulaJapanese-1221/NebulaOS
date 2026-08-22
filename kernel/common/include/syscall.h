#ifndef NEBULAOS_SYSCALL_H
#define NEBULAOS_SYSCALL_H

#include "../include/nebula.h"
#include "../include/stdint.h"

#define SYS_WRITE 1
#define SYS_READ 2
#define SYS_OPEN 3
#define SYS_CLOSE 4
#define SYS_EXIT 5
#define SYS_GETPID 6
#define SYS_YIELD 7

void syscall_init(void);
uint64_t syscall_dispatch(uint64_t num, uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5);

#endif