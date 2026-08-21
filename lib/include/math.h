// NebulaOS - Math Library Header
// ==============================
//
// Math functions

#ifndef NEBULAOS_LIB_MATH_H
#define NEBULAOS_LIB_MATH_H

#ifdef __cplusplus
extern "C" {
#endif

// Absolute value
int abs(int x);
long labs(long x);

// Min/Max
int min(int a, int b);
int max(int a, int b);
long lmin(long a, long b);
long lmax(long a, long b);

// Floating point
float fabs(float x);
double fabsl(double x);
float fmod(float x, float y);
double fmodl(double x, double y);

// Square root (integer)
int isqrt(int x);

// Random number generator
#define RAND_MAX 0x7FFFFFFF
void srand(unsigned int seed);
int rand();

#ifdef __cplusplus
}
#endif

#endif // NEBULAOS_LIB_MATH_H
