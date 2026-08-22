// NebulaOS - C++ new/delete
// ===========================
//
// Minimal C++ allocation operators

#include <new>
#include "../../lib/include/stdlib.h"

void* operator new(size_t size) {
    return malloc(size);
}

void* operator new[](size_t size) {
    return malloc(size);
}

void operator delete(void* ptr) noexcept {
    free(ptr);
}

void operator delete[](void* ptr) noexcept {
    free(ptr);
}

void* operator new(size_t, void* ptr) noexcept {
    return ptr;
}

void* operator new[](size_t, void* ptr) noexcept {
    return ptr;
}