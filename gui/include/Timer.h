// NebulaOS GUI - Timer Subsystem
// ===============================
//
// Timer management for the GUI system

#ifndef NEBULAOS_GUI_TIMER_H
#define NEBULAOS_GUI_TIMER_H

#include "../include/GuiTypes.h"

namespace NebulaOS {
namespace GUI {

// Timer callback type
using TimerCallback = void (*)(uint32_t timerId, uint32_t elapsed, void* userData);

// Timer information
struct TimerInfo {
    TimerID id;
    WindowHandle hwnd;
    uint32_t interval;  // Milliseconds
    uint32_t elapsed;
    TimerCallback callback;
    void* userData;
    bool active;
    bool oneshot;
};

// Initialize timer subsystem
bool InitializeTimers();

// Shutdown timer subsystem
void ShutdownTimers();

// Create a timer
TimerID CreateTimer(WindowHandle hwnd, uint32_t interval, TimerCallback callback, void* userData = nullptr, bool oneshot = false);

// Destroy a timer
void DestroyTimer(TimerID timerId);

// Start a timer
void StartTimer(TimerID timerId);

// Stop a timer
void StopTimer(TimerID timerId);

// Pause a timer
void PauseTimer(TimerID timerId);

// Resume a timer
void ResumeTimer(TimerID timerId);

// Reset a timer
void ResetTimer(TimerID timerId);

// Check if timer exists
bool IsTimerActive(TimerID timerId);

// Process all timers
void ProcessTimers();

// Get timer information
bool GetTimerInfo(TimerID timerId, TimerInfo& info);

// Get current time
uint32_t GetTickCount();
uint32_t GetElapsedTime(uint32_t startTime);

// Sleep functions
void TimerSleep(uint32_t milliseconds);

// Serialization
void TimerToString(TimerID timerId, char* buffer, size_t size);

} // namespace GUI
} // namespace NebulaOS

#endif // NEBULAOS_GUI_TIMER_H
