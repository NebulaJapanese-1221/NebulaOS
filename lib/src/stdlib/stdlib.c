// NebulaOS - stdlib implementation
// ==================================
//
// Minimal standard library

#include "../../lib/include/stdlib.h"
#include "../../lib/include/string.h"
#include "../../kernel/common/include/nebula.h"

static unsigned int seed = 1;

int atoi(const char* str) {
    return (int)strtol(str, NULL, 10);
}

long atol(const char* str) {
    return strtol(str, NULL, 10);
}

long strtol(const char* str, char** endptr, int base) {
    const char* p = str;
    long result = 0;
    int sign = 1;
    
    while (*p == ' ' || *p == '\t') p++;
    
    if (*p == '-') { sign = -1; p++; }
    else if (*p == '+') p++;
    
    if (base == 0) {
        if (p[0] == '0') {
            if (p[1] == 'x' || p[1] == 'X') base = 16;
            else base = 8;
        } else base = 10;
    }
    
    while (*p) {
        int digit;
        if (*p >= '0' && *p <= '9') digit = *p - '0';
        else if (*p >= 'a' && *p <= 'z') digit = *p - 'a' + 10;
        else if (*p >= 'A' && *p <= 'Z') digit = *p - 'A' + 10;
        else break;
        
        if (digit >= base) break;
        
        result = result * base + digit;
        p++;
    }
    
    if (endptr) *endptr = (char*)p;
    return sign * result;
}

unsigned long strtoul(const char* str, char** endptr, int base) {
    const char* p = str;
    unsigned long result = 0;
    
    while (*p == ' ' || *p == '\t') p++;
    if (*p == '+') p++;
    
    if (base == 0) {
        if (p[0] == '0') {
            if (p[1] == 'x' || p[1] == 'X') base = 16;
            else base = 8;
        } else base = 10;
    }
    
    while (*p) {
        int digit;
        if (*p >= '0' && *p <= '9') digit = *p - '0';
        else if (*p >= 'a' && *p <= 'z') digit = *p - 'a' + 10;
        else if (*p >= 'A' && *p <= 'Z') digit = *p - 'A' + 10;
        else break;
        
        if (digit >= base) break;
        result = result * base + digit;
        p++;
    }
    
    if (endptr) *endptr = (char*)p;
    return result;
}

int abs(int x) {
    return x < 0 ? -x : x;
}

long labs(long x) {
    return x < 0 ? -x : x;
}

void srand(unsigned int s) {
    seed = s;
}

int rand(void) {
    seed = seed * 1103515245 + 12345;
    return (int)(seed / 65536) % 32768;
}

void qsort(void* base, size_t nmemb, size_t size, int (*compar)(const void*, const void*)) {
    if (nmemb < 2) return;
    
    char* arr = (char*)base;
    for (size_t i = 0; i < nmemb - 1; i++) {
        for (size_t j = 0; j < nmemb - i - 1; j++) {
            if (compar(arr + j * size, arr + (j + 1) * size) > 0) {
                char tmp[256];
                memcpy(tmp, arr + j * size, size);
                memcpy(arr + j * size, arr + (j + 1) * size, size);
                memcpy(arr + (j + 1) * size, tmp, size);
            }
        }
    }
}

void* bsearch(const void* key, const void* base, size_t nmemb, size_t size, int (*compar)(const void*, const void*)) {
    size_t low = 0, high = nmemb;
    const char* arr = (const char*)base;
    
    while (low < high) {
        size_t mid = low + (high - low) / 2;
        int cmp = compar(key, arr + mid * size);
        if (cmp == 0) return (void*)(arr + mid * size);
        else if (cmp < 0) high = mid;
        else low = mid + 1;
    }
    
    return NULL;
}

void exit(int status) {
    (void)status;
    kernel_panic("exit() called");
}