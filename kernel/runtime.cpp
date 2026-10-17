// Freestanding C runtime routines used by the kernel.

#include <cstddef>
#include <cstdint>

extern "C" {

void* memset(void* destination, int value, std::size_t count) {
    auto* out = static_cast<volatile unsigned char*>(destination);
    for (std::size_t i = 0; i < count; ++i) {
        out[i] = static_cast<unsigned char>(value);
    }
    return destination;
}

void* memcpy(void* destination, const void* source, std::size_t count) {
    auto* out = static_cast<volatile unsigned char*>(destination);
    const auto* in = static_cast<const volatile unsigned char*>(source);
    for (std::size_t i = 0; i < count; ++i) {
        out[i] = in[i];
    }
    return destination;
}

void* memmove(void* destination, const void* source, std::size_t count) {
    auto* out = static_cast<volatile unsigned char*>(destination);
    const auto* in = static_cast<const volatile unsigned char*>(source);
    if (reinterpret_cast<std::uintptr_t>(out) <
        reinterpret_cast<std::uintptr_t>(in)) {
        for (std::size_t i = 0; i < count; ++i) {
            out[i] = in[i];
        }
    } else {
        while (count > 0) {
            --count;
            out[count] = in[count];
        }
    }
    return destination;
}

int memcmp(const void* lhs, const void* rhs, std::size_t count) {
    const auto* left = static_cast<const volatile unsigned char*>(lhs);
    const auto* right = static_cast<const volatile unsigned char*>(rhs);
    for (std::size_t i = 0; i < count; ++i) {
        if (left[i] != right[i]) {
            return static_cast<int>(left[i]) - static_cast<int>(right[i]);
        }
    }
    return 0;
}

std::size_t strlen(const char* text) {
    std::size_t length = 0;
    while (text[length] != '\0') {
        ++length;
    }
    return length;
}

int strcmp(const char* lhs, const char* rhs) {
    while (*lhs != '\0' && *lhs == *rhs) {
        ++lhs;
        ++rhs;
    }
    return static_cast<int>(static_cast<unsigned char>(*lhs)) -
           static_cast<int>(static_cast<unsigned char>(*rhs));
}

char* strcpy(char* destination, const char* source) {
    char* result = destination;
    while ((*destination++ = *source++) != '\0') {
    }
    return result;
}

char* strncpy(char* destination, const char* source, std::size_t count) {
    std::size_t i = 0;
    for (; i < count && source[i] != '\0'; ++i) {
        destination[i] = source[i];
    }
    for (; i < count; ++i) {
        destination[i] = '\0';
    }
    return destination;
}

}
