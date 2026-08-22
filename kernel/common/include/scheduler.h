#ifndef NEBULAOS_SCHEDULER_H
#define NEBULAOS_SCHEDULER_H

#include "../include/nebula.h"
#include "../include/stdint.h"

#define SCHEDULER_TIME_SLICE 10

void scheduler_init(void);
void scheduler_add(uint32_t pid);
void scheduler_remove(uint32_t pid);
void scheduler_tick(void);

#endif