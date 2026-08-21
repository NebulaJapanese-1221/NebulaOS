// NebulaOS - Time Library Header
// ==============================
//
// Time functions

#ifndef NEBULAOS_LIB_TIME_H
#define NEBULAOS_LIB_TIME_H

#include "stdint.h"

#ifdef __cplusplus
extern "C" {
#endif

// Time types
typedef long time_t;
typedef long clock_t;

// Time structure
struct tm {
    int tm_sec;    // Seconds (0-60)
    int tm_min;    // Minutes (0-59)
    int tm_hour;   // Hours (0-23)
    int tm_mday;   // Day of month (1-31)
    int tm_mon;    // Month (0-11)
    int tm_year;   // Year (since 1900)
    int tm_wday;   // Day of week (0-6)
    int tm_yday;   // Day of year (0-365)
    int tm_isdst;  // Daylight saving time flag
};

// Time functions
time_t time(time_t* timer);
clock_t clock();

// Sleep functions
void sleep(int seconds);
void usleep(unsigned int microseconds);

// Time conversion
struct tm* localtime(const time_t* timer);
struct tm* gmtime(const time_t* timer);

// Time formatting
size_t strftime(char* str, size_t maxsize, const char* format, const struct tm* timeptr);

#ifdef __cplusplus
}
#endif

#endif // NEBULAOS_LIB_TIME_H
