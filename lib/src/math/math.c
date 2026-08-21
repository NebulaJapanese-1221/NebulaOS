// NebulaOS - Math Library
// ======================
//
// Math functions for the kernel

#include "math.h"

// -----------------------------------------------------------------------------
// abs - Absolute value
// -----------------------------------------------------------------------------

int abs(int x) {
    return x < 0 ? -x : x;
}

long labs(long x) {
    return x < 0 ? -x : x;
}

// -----------------------------------------------------------------------------
// min/max
// -----------------------------------------------------------------------------

int min(int a, int b) {
    return a < b ? a : b;
}

int max(int a, int b) {
    return a > b ? a : b;
}

long lmin(long a, long b) {
    return a < b ? a : b;
}

long lmax(long a, long b) {
    return a > b ? a : b;
}

// -----------------------------------------------------------------------------
// Floating point functions (stub implementations)
// -----------------------------------------------------------------------------

float fabs(float x) {
    return x < 0 ? -x : x;
}

double fabsl(double x) {
    return x < 0 ? -x : x;
}

float fmod(float x, float y) {
    // Simple implementation
    int quotient = (int)(x / y);
    return x - quotient * y;
}

double fmodl(double x, double y) {
    long quotient = (long)(x / y);
    return x - quotient * y;
}

// -----------------------------------------------------------------------------
// Square root (integer approximation)
// -----------------------------------------------------------------------------

int isqrt(int x) {
    if (x <= 0) return 0;
    
    int result = 0;
    int bit = 1 << 30;
    
    while (bit > x) {
        bit >>= 2;
    }
    
    while (bit != 0) {
        if (x >= result + bit) {
            x -= result + bit;
            result = (result >> 1) + bit;
        } else {
            result >>= 1;
        }
        bit >>= 2;
    }
    
    return result;
}

// -----------------------------------------------------------------------------
// Random number generator (simple LCG)
// -----------------------------------------------------------------------------

static unsigned int random_seed = 12345;

void srand(unsigned int seed) {
    random_seed = seed;
}

int rand() {
    random_seed = random_seed * 1103515245 + 12345;
    return (int)(random_seed & RAND_MAX);
}
