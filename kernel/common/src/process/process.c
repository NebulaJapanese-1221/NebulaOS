// NebulaOS - Process Management
// ================================
//
// Process control blocks and context switching

#include "../../common/include/process.h"
#include "../../common/include/nebula.h"
#include "../../common/include/stdint.h"
#include "../../common/include/scheduler.h"
#include "../../lib/include/string.h"
#include "../../kernel/common/include/idt.h"
#include "../../kernel/common/include/memory.h"

static process_t process_table[MAX_PROCESSES];
static uint32_t next_pid = 1;
static process_t* current_process = NULL;
static bool process_initialized = false;

void process_init(void) {
    memset(process_table, 0, sizeof(process_table));
    
    for (int i = 0; i < MAX_PROCESSES; i++) {
        process_table[i].pid = 0;
        process_table[i].state = PROC_ZOMBIE;
        process_table[i].next = NULL;
    }
    
    current_process = NULL;
    process_initialized = true;
}

int process_create(void* entry, uint32_t stack_size) {
    if (!process_initialized) return -1;
    if (!entry || stack_size == 0) return -1;
    
    for (int i = 0; i < MAX_PROCESSES; i++) {
        if (process_table[i].state == PROC_ZOMBIE) {
            process_t* proc = &process_table[i];
            
            proc->pid = next_pid++;
            proc->state = PROC_READY;
            proc->entry_point = entry;
            proc->stack_size = ALIGN_UP(stack_size, 16);
            proc->stack_base = (uint64_t)malloc(proc->stack_size) + proc->stack_size;
            
            memset(&proc->context, 0, sizeof(cpu_context_t));
            proc->context.rip = (uint64_t)entry;
            proc->context.rsp = proc->stack_base;
            proc->context.cs = 0x08;
            proc->context.ss = 0x10;
            proc->context.rflags = 0x202;
            proc->context.rdi = 0;
            proc->context.rsi = 0;
            proc->context.rdx = 0;
            proc->context.rcx = 0;
            proc->context.rbx = 0;
            proc->context.rax = 0;
            proc->context.rbp = proc->stack_base;
            
            for (int f = 0; f < 8; f++) {
                proc->fd_table[f] = -1;
            }
            
            proc->next = NULL;
            
            if (!current_process) {
                current_process = proc;
            }
            
            scheduler_add(proc->pid);
            
            return (int)proc->pid;
        }
    }
    
    return -1;
}

void process_destroy(uint32_t pid) {
    if (!process_initialized) return;
    
    for (int i = 0; i < MAX_PROCESSES; i++) {
        if (process_table[i].pid == pid) {
            process_table[i].state = PROC_ZOMBIE;
            if (current_process == &process_table[i]) {
                current_process = NULL;
            }
            return;
        }
    }
}

process_t* process_get_current(void) {
    return current_process;
}

void process_set_current(uint32_t pid) {
    for (int i = 0; i < MAX_PROCESSES; i++) {
        if (process_table[i].pid == pid) {
            current_process = &process_table[i];
            return;
        }
    }
}

void process_context_switch(cpu_context_t* regs) {
    if (!current_process) return;
    
    current_process->state = PROC_READY;
    memcpy(&current_process->context, regs, sizeof(cpu_context_t));
}

process_t* process_next(void) {
    uint32_t current_pid = current_process ? current_process->pid : 0;
    int start = -1;
    
    for (int i = 0; i < MAX_PROCESSES; i++) {
        if (process_table[i].pid == current_pid) {
            start = i;
            break;
        }
    }
    
    if (start < 0) start = 0;
    
    for (int i = 1; i <= MAX_PROCESSES; i++) {
        int idx = (start + i) % MAX_PROCESSES;
        if (process_table[idx].state == PROC_READY && process_table[idx].pid != current_pid) {
            return &process_table[idx];
        }
    }
    
    return current_process;
}

uint32_t process_get_count(void) {
    uint32_t count = 0;
    for (int i = 0; i < MAX_PROCESSES; i++) {
        if (process_table[i].state != PROC_ZOMBIE) {
            count++;
        }
    }
    return count;
}

process_state_t process_get_state(uint32_t index) {
    if (index < MAX_PROCESSES) {
        return process_table[index].state;
    }
    return PROC_ZOMBIE;
}

uint32_t process_get_pid(uint32_t index) {
    if (index < MAX_PROCESSES && process_table[index].state != PROC_ZOMBIE) {
        return process_table[index].pid;
    }
    return 0;
}

void* process_get_entry_point(uint32_t index) {
    if (index < MAX_PROCESSES && process_table[index].state != PROC_ZOMBIE) {
        return process_table[index].entry_point;
    }
    return NULL;
}