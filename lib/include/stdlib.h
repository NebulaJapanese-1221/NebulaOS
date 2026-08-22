#ifndef NEBULAOS_STDLIB_H
#define NEBULAOS_STDLIB_H

#include "stdint.h"

int atoi(const char* str);
long atol(const char* str);
long strtol(const char* str, char** endptr, int base);
unsigned long strtoul(const char* str, char** endptr, int base);

int abs(int x);
long labs(long x);

void srand(unsigned int seed);
int rand(void);

void qsort(void* base, size_t nmemb, size_t size, int (*compar)(const void*, const void*));
void* bsearch(const void* key, const void* base, size_t nmemb, size_t size, int (*compar)(const void*, const void*));

void exit(int status);

#endif