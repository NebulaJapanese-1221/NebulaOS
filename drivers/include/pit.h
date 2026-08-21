// NebulaOS - PIT (Programmable Interval Timer) Driver
// ===================================================
//
// 8253/8254 PIT driver header

#ifndef NEBULAOS_DRIVERS_PIT_H
#define NEBULAOS_DRIVERS_PIT_H

#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/nebula.h"

// PIT ports
#define PIT_CHANNEL0_PORT 0x40
#define PIT_CHANNEL1_PORT 0x41
#define PIT_CHANNEL2_PORT 0x42
#define PIT_COMMAND_PORT  0x43

// PIT channels
#define PIT_CHANNEL0 0  // System timer
#define PIT_CHANNEL1 1  // Memory refresh
#define PIT_CHANNEL2 2  // Speaker tone

// PIT command register bits
#define PIT_CMD_CHANNEL_MASK  0xC0  // Channel selection (bits 7-6)
#define PIT_CMD_ACCESS_MASK   0x30  // Access mode (bits 5-4)
#define PIT_CMD_MODE_MASK     0x0E  // Mode (bits 3-1)
#define PIT_CMD_BCD_MASK      0x01  // BCD mode (bit 0)

// Access modes
#define PIT_ACCESS_LATCH_COUNT  0x00  // Latch count value
#define PIT_ACCESS_LOW_BYTE     0x10  // Low byte only
#define PIT_ACCESS_HIGH_BYTE    0x20  // High byte only
#define PIT_ACCESS_WORD         0x30  // Low then high byte

// Modes
#define PIT_MODE_INTERRUPT      0x00  // Interrupt on terminal count
#define PIT_MODE_ONESHOT        0x02  // One-shot
#define PIT_MODE_RATE_GEN       0x04  // Rate generator
#define PIT_MODE_SQUARE_WAVE    0x06  // Square wave generator
#define PIT_MODE_SOFT_TRIG      0x08  // Software triggered strobe
#define PIT_MODE_HARD_TRIG      0x0A  // Hardware triggered strobe

// Clock frequency (1.193182 MHz)
#define PIT_BASE_FREQ 1193182

// Timer tick rate (100 Hz)
#define TIMER_FREQ 100
#define TIMER_TICK_US (1000000 / TIMER_FREQ)

// Timer state
typedef struct {
    uint32_t tickCount;
    uint32_t milliseconds;
    uint32_t seconds;
    uint32_t minutes;
    uint32_t hours;
} timer_state_t;

// Timer callback
typedef void (*timer_handler_t)(uint32_t tickCount);

// PIT functions
void pit_init(uint32_t frequency);
void pit_set_frequency(uint32_t channel, uint32_t frequency);
uint32_t pit_get_frequency(uint32_t channel);

// Timer functions
void timer_init();
timer_state_t timer_get_state();
void timer_set_handler(timer_handler_t handler);

// Sleep functions
void sleep(uint32_t milliseconds);
void usleep(uint32_t microseconds);

// Get current tick count
uint32_t timer_get_ticks();

// Get current time in milliseconds
uint32_t timer_get_milliseconds();

// IRQ handler
void pit_irq_handler();

#endif // NEBULAOS_DRIVERS_PIT_H
