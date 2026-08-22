// NebulaOS - Round-Robin Scheduler
// ==================================
//
// Simple round-robin process scheduler

#include "../../common/include/scheduler.h"
#include "../../common/include/process.h"
#include "../../common/include/nebula.h"
#include "../../common/include/stdint.h"

static uint32_t scheduler_queue[MAX_PROCESSES];
static int queue_head = 0;
static int queue_tail = 0;
static int queue_count = 0;
static uint32_t current_scheduled_pid = 0;
static uint32_t tick_counter = 0;
static bool scheduler_initialized = false;

void scheduler_init(void) {
    for (int i = 0; i < MAX_PROCESSES; i++) {
        scheduler_queue[i] = 0;
    }
    queue_head = 0;
    queue_tail = 0;
    queue_count = 0;
    current_scheduled_pid = 0;
    tick_counter = 0;
    scheduler_initialized = true;
}

void scheduler_add(uint32_t pid) {
    if (!scheduler_initialized) return;
    if (queue_count >= MAX_PROCESSES) return;
    
    scheduler_queue[queue_tail] = pid;
    queue_tail = (queue_tail + 1) % MAX_PROCESSES;
    queue_count++;
}

void scheduler_remove(uint32_t pid) {
    if (!scheduler_initialized) return;
    
    for (int i = 0; i < queue_count; i++) {
        int idx = (queue_head + i) % MAX_PROCESSES;
        if (scheduler_queue[idx] == pid) {
            for (int j = i; j < queue_count - 1; j++) {
                int cur = (queue_head + j) % MAX_PROCESSES;
                int nxt = (queue_head + j + 1) % MAX_PROCESSES;
                scheduler_queue[cur] = scheduler_queue[nxt];
            }
            queue_tail = (queue_tail - 1 + MAX_PROCESSES) % MAX_PROCESSES;
            queue_count--;
            return;
        }
    }
}

void scheduler_tick(void) {
    if (!scheduler_initialized || queue_count == 0) return;
    
    tick_counter++;
    if (tick_counter < SCHEDULER_TIME_SLICE) return;
    
    tick_counter = 0;
    
    uint32_t old_pid = scheduler_queue[queue_head];
    queue_head = (queue_head + 1) % MAX_PROCESSES;
    queue_tail = (queue_tail + 1) % MAX_PROCESSES;
    scheduler_queue[queue_tail] = old_pid;
    
    current_scheduled_pid = scheduler_queue[queue_head];
    process_set_current(current_scheduled_pid);
    process_t* cur = process_get_current();
    if (cur) cur->state = PROC_RUNNING;
}