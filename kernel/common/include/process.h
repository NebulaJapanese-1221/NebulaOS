#ifndef NEBULAOS_PROCESS_H
#define NEBULAOS_PROCESS_H

#include "../include/nebula.h"
#include "../include/stdint.h"

#define MAX_PROCESSES 16

typedef enum {
    PROC_READY = 0,
    PROC_RUNNING = 1,
    PROC_BLOCKED = 2,
    PROC_ZOMBIE = 3
} process_state_t;

typedef struct cpu_context {
    uint64_t r15;
    uint64_t r14;
    uint64_t r13;
    uint64_t r12;
    uint64_t r11;
    uint64_t r10;
    uint64_t r9;
    uint64_t r8;
    uint64_t rbp;
    uint64_t rdi;
    uint64_t rsi;
    uint64_t rdx;
    uint64_t rcx;
    uint64_t rbx;
    uint64_t rax;
    uint64_t rip;
    uint64_t cs;
    uint64_t rflags;
    uint64_t rsp;
    uint64_t ss;
} cpu_context_t;

typedef struct process {
    uint32_t pid;
    process_state_t state;
    cpu_context_t context;
    uint64_t stack_base;
    uint32_t stack_size;
    void* entry_point;
    int fd_table[8];
    struct process* next;
} process_t;

void process_init(void);
int process_create(void* entry, uint32_t stack_size);
void process_destroy(uint32_t pid);
process_t* process_get_current(void);
void process_set_current(uint32_t pid);
void process_context_switch(cpu_context_t* regs);

#endif