// NebulaOS - PIT (Programmable Interval Timer) Driver
// ====================================================
//
// Implementation of 8253/8254 PIT driver

#include "../include/pit.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/idt.h"

// Timer state
static volatile timer_state_t timer_state = {0, 0, 0, 0, 0};
static timer_handler_t timer_callback = NULL;
static uint32_t timer_frequency = TIMER_FREQ;

// -----------------------------------------------------------------------------
// I/O functions
// -----------------------------------------------------------------------------

static inline uint8_t inb(uint16_t port) {
    uint8_t value;
    __asm__ __volatile__("inb %1, %0" : "=a"(value) : "dN"(port));
    return value;
}

static inline void outb(uint16_t port, uint8_t value) {
    __asm__ __volatile__("outb %0, %1" : : "a"(value), "dN"(port));
}

// -----------------------------------------------------------------------------
// Initialize PIT channel
// -----------------------------------------------------------------------------

void pit_init(uint32_t frequency) {
    timer_frequency = frequency;
    pit_set_frequency(PIT_CHANNEL0, frequency);
}

// -----------------------------------------------------------------------------
// Set PIT frequency for a channel
// -----------------------------------------------------------------------------

void pit_set_frequency(uint32_t channel, uint32_t frequency) {
    if (frequency == 0) return;
    
    // Calculate divisor
    uint32_t divisor = PIT_BASE_FREQ / frequency;
    
    if (divisor > 65535) {
        divisor = 65535;
    }
    
    // Build command byte
    uint8_t command = 0;
    command |= (channel & 0x03) << 6;  // Channel selection
    command |= PIT_ACCESS_WORD;         // 16-bit access
    command |= PIT_MODE_RATE_GEN;       // Rate generator mode
    command |= 0x00;                    // 16-bit binary
    
    // Send command
    outb(PIT_COMMAND_PORT, command);
    
    // Send divisor (low byte then high byte)
    uint8_t low = divisor & 0xFF;
    uint8_t high = (divisor >> 8) & 0xFF;
    
    outb(PIT_CHANNEL0_PORT + channel, low);
    outb(PIT_CHANNEL0_PORT + channel, high);
}

// -----------------------------------------------------------------------------
// Get PIT frequency for a channel
// -----------------------------------------------------------------------------

uint32_t pit_get_frequency(uint32_t channel) {
    // Latch count value
    uint8_t command = PIT_ACCESS_WORD | (channel & 0x03) << 6;
    outb(PIT_COMMAND_PORT, command);
    
    // Read low byte
    uint8_t low = inb(PIT_CHANNEL0_PORT + channel);
    
    // Read high byte
    uint8_t high = inb(PIT_CHANNEL0_PORT + channel);
    
    // Calculate divisor
    uint32_t divisor = low | (high << 8);
    
    if (divisor == 0) return 0;
    
    return PIT_BASE_FREQ / divisor;
}

// -----------------------------------------------------------------------------
// Timer initialization
// -----------------------------------------------------------------------------

void timer_init() {
    // Initialize PIT channel 0 to 100 Hz
    pit_init(TIMER_FREQ);
    
    // Register IRQ handler
    register_interrupt_handler(IRQ0, (interrupt_handler_t)pit_irq_handler);
    
    // Reset timer state
    timer_state.tickCount = 0;
    timer_state.milliseconds = 0;
    timer_state.seconds = 0;
    timer_state.minutes = 0;
    timer_state.hours = 0;
}

// -----------------------------------------------------------------------------
// Get timer state
// -----------------------------------------------------------------------------

timer_state_t timer_get_state() {
    return timer_state;
}

// -----------------------------------------------------------------------------
// Set timer callback
// -----------------------------------------------------------------------------

void timer_set_handler(timer_handler_t handler) {
    timer_callback = handler;
}

// -----------------------------------------------------------------------------
// IRQ handler for PIT (IRQ0)
// -----------------------------------------------------------------------------

void pit_irq_handler() {
    // Increment tick count
    timer_state.tickCount++;
    
    // Update time
    timer_state.milliseconds += TIMER_TICK_US / 1000;
    if (timer_state.milliseconds >= 1000) {
        timer_state.milliseconds = 0;
        timer_state.seconds++;
        if (timer_state.seconds >= 60) {
            timer_state.seconds = 0;
            timer_state.minutes++;
            if (timer_state.minutes >= 60) {
                timer_state.minutes = 0;
                timer_state.hours++;
                if (timer_state.hours >= 24) {
                    timer_state.hours = 0;
                }
            }
        }
    }
    
    // Call callback if registered
    if (timer_callback) {
        timer_callback(timer_state.tickCount);
    }
    
    // Send EOI to PIC
    outb(0x20, 0x20);
}

// -----------------------------------------------------------------------------
// Get current tick count
// -----------------------------------------------------------------------------

uint32_t timer_get_ticks() {
    return timer_state.tickCount;
}

// -----------------------------------------------------------------------------
// Get current time in milliseconds
// -----------------------------------------------------------------------------

uint32_t timer_get_milliseconds() {
    return timer_state.tickCount * (1000 / TIMER_FREQ);
}

// -----------------------------------------------------------------------------
// Sleep for milliseconds
// -----------------------------------------------------------------------------

void sleep(uint32_t milliseconds) {
    uint32_t start = timer_get_milliseconds();
    uint32_t end = start + milliseconds;
    
    // Handle overflow
    if (end < start) {
        // Wait until overflow
        while (timer_get_milliseconds() >= start) {
            __asm__ __volatile__("pause");
        }
        // Wait until end
        while (timer_get_milliseconds() < end) {
            __asm__ __volatile__("pause");
        }
    } else {
        while (timer_get_milliseconds() < end) {
            __asm__ __volatile__("pause");
        }
    }
}

// -----------------------------------------------------------------------------
// Sleep for microseconds
// -----------------------------------------------------------------------------

void usleep(uint32_t microseconds) {
    uint32_t start = timer_get_ticks();
    uint32_t ticks = microseconds / TIMER_TICK_US;
    uint32_t end = start + ticks;
    
    if (end < start) {
        while (timer_get_ticks() >= start) {
            __asm__ __volatile__("pause");
        }
        while (timer_get_ticks() < end) {
            __asm__ __volatile__("pause");
        }
    } else {
        while (timer_get_ticks() < end) {
            __asm__ __volatile__("pause");
        }
    }
}
