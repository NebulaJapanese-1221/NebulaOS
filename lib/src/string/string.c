// NebulaOS - String Library
// ========================
//
// String manipulation functions for the kernel

#include "string.h"
#include "../include/stdint.h"
#include "../../kernel/common/include/memory.h"

// -----------------------------------------------------------------------------
// strlen - Get string length
// -----------------------------------------------------------------------------

size_t strlen(const char* str) {
    size_t len = 0;
    while (str[len] != '\0') {
        len++;
    }
    return len;
}

// -----------------------------------------------------------------------------
// strcpy - Copy string
// -----------------------------------------------------------------------------

char* strcpy(char* dest, const char* src) {
    size_t i = 0;
    while (src[i] != '\0') {
        dest[i] = src[i];
        i++;
    }
    dest[i] = '\0';
    return dest;
}

// -----------------------------------------------------------------------------
// strncpy - Copy string (with length limit)
// -----------------------------------------------------------------------------

char* strncpy(char* dest, const char* src, size_t n) {
    size_t i = 0;
    while (i < n && src[i] != '\0') {
        dest[i] = src[i];
        i++;
    }
    while (i < n) {
        dest[i] = '\0';
        i++;
    }
    return dest;
}

// -----------------------------------------------------------------------------
// strcat - Concatenate strings
// -----------------------------------------------------------------------------

char* strcat(char* dest, const char* src) {
    size_t i = strlen(dest);
    size_t j = 0;
    while (src[j] != '\0') {
        dest[i + j] = src[j];
        j++;
    }
    dest[i + j] = '\0';
    return dest;
}

// -----------------------------------------------------------------------------
// strncat - Concatenate strings (with length limit)
// -----------------------------------------------------------------------------

char* strncat(char* dest, const char* src, size_t n) {
    size_t i = strlen(dest);
    size_t j = 0;
    while (j < n && src[j] != '\0') {
        dest[i + j] = src[j];
        j++;
    }
    dest[i + j] = '\0';
    return dest;
}

// -----------------------------------------------------------------------------
// strcmp - Compare strings
// -----------------------------------------------------------------------------

int strcmp(const char* s1, const char* s2) {
    size_t i = 0;
    while (s1[i] != '\0' && s2[i] != '\0') {
        if (s1[i] != s2[i]) {
            return s1[i] < s2[i] ? -1 : 1;
        }
        i++;
    }
    if (s1[i] == '\0' && s2[i] == '\0') {
        return 0;
    }
    return s1[i] == '\0' ? -1 : 1;
}

// -----------------------------------------------------------------------------
// strncmp - Compare strings (with length limit)
// -----------------------------------------------------------------------------

int strncmp(const char* s1, const char* s2, size_t n) {
    size_t i = 0;
    while (i < n && s1[i] != '\0' && s2[i] != '\0') {
        if (s1[i] != s2[i]) {
            return s1[i] < s2[i] ? -1 : 1;
        }
        i++;
    }
    if (i == n) {
        return 0;
    }
    if (s1[i] == '\0' && s2[i] == '\0') {
        return 0;
    }
    return s1[i] == '\0' ? -1 : 1;
}

// -----------------------------------------------------------------------------
// strchr - Find character in string
// -----------------------------------------------------------------------------

char* strchr(char* str, int c) {
    while (*str != '\0') {
        if (*str == (char)c) {
            return str;
        }
        str++;
    }
    return *str == (char)c ? str : NULL;
}

// -----------------------------------------------------------------------------
// strrchr - Find last occurrence of character in string
// -----------------------------------------------------------------------------

char* strrchr(char* str, int c) {
    char* last = NULL;
    while (*str != '\0') {
        if (*str == (char)c) {
            last = str;
        }
        str++;
    }
    return *str == (char)c ? str : last;
}

// -----------------------------------------------------------------------------
// snprintf - Formatted string output (simplified version)
// -----------------------------------------------------------------------------

#include "stdarg.h"

static void itoa(char* buffer, int value, int base, bool upper) {
    static const char* digits = "0123456789abcdef";
    static const char* digits_upper = "0123456789ABCDEF";
    const char* use_digits = upper ? digits_upper : digits;
    
    if (value == 0) {
        buffer[0] = '0';
        buffer[1] = '\0';
        return;
    }
    
    bool negative = false;
    if (value < 0) {
        negative = true;
        value = -value;
    }
    
    char temp[32];
    int i = 0;
    while (value > 0) {
        temp[i++] = use_digits[value % base];
        value /= base;
    }
    
    if (negative) {
        temp[i++] = '-';
    }
    
    for (int j = 0; j < i; j++) {
        buffer[j] = temp[i - 1 - j];
    }
    buffer[i] = '\0';
}

static void utoa(char* buffer, unsigned int value, int base, bool upper) {
    static const char* digits = "0123456789abcdef";
    static const char* digits_upper = "0123456789ABCDEF";
    const char* use_digits = upper ? digits_upper : digits;
    
    if (value == 0) {
        buffer[0] = '0';
        buffer[1] = '\0';
        return;
    }
    
    char temp[32];
    int i = 0;
    while (value > 0) {
        temp[i++] = use_digits[value % base];
        value /= base;
    }
    
    for (int j = 0; j < i; j++) {
        buffer[j] = temp[i - 1 - j];
    }
    buffer[i] = '\0';
}

int snprintf(char* buffer, size_t size, const char* format, ...) {
    if (size == 0) return 0;
    
    va_list args;
    va_start(args, format);
    
    int written = 0;
    size_t pos = 0;
    
    while (*format != '\0' && pos < size - 1) {
        if (*format == '%') {
            format++;
            bool long_format = false;
            bool upper = false;
            int width = 0;
            
            // Parse flags
            while (*format == '0' || *format == '-' || *format == ' ' || *format == '+') {
                format++;
            }
            
            // Parse width
            while (*format >= '0' && *format <= '9') {
                width = width * 10 + (*format - '0');
                format++;
            }
            
            // Parse precision
            if (*format == '.') {
                format++;
                while (*format >= '0' && *format <= '9') {
                    format++;
                }
            }
            
            // Parse length modifier
            if (*format == 'l') {
                long_format = true;
                format++;
                if (*format == 'l') {
                    format++;
                }
            } else if (*format == 'h') {
                format++;
            }
            
            // Parse conversion specifier
            char spec = *format;
            format++;
            
            if (spec == '%') {
                buffer[pos++] = '%';
                written++;
                continue;
            }
            
            // Handle upper case hex
            if (spec == 'X') {
                upper = true;
                spec = 'x';
            } else if (spec == 'A' || spec == 'E' || spec == 'F' || spec == 'G') {
                upper = true;
            }
            
            // Process the argument
            char temp[32];
            switch (spec) {
                case 'd':
                case 'i':
                    {
                        int value = long_format ? va_arg(args, long) : va_arg(args, int);
                        itoa(temp, value, 10, upper);
                        break;
                    }
                case 'u':
                    {
                        unsigned int value = long_format ? va_arg(args, unsigned long) : va_arg(args, unsigned int);
                        utoa(temp, value, 10, upper);
                        break;
                    }
                case 'x':
                    {
                        unsigned int value = long_format ? va_arg(args, unsigned long) : va_arg(args, unsigned int);
                        utoa(temp, value, 16, upper);
                        break;
                    }
                case 'o':
                    {
                        unsigned int value = long_format ? va_arg(args, unsigned long) : va_arg(args, unsigned int);
                        utoa(temp, value, 8, upper);
                        break;
                    }
                case 's':
                    {
                        const char* str = va_arg(args, const char*);
                        if (!str) str = "(null)";
                        strcpy(temp, str);
                        break;
                    }
                case 'c':
                    {
                        char c = (char)va_arg(args, int);
                        temp[0] = c;
                        temp[1] = '\0';
                        break;
                    }
                case 'p':
                    {
                        void* ptr = va_arg(args, void*);
                        utoa(temp, (unsigned int)(unsigned long)ptr, 16, false);
                        break;
                    }
                default:
                    buffer[pos++] = '%';
                    buffer[pos++] = spec;
                    written += 2;
                    continue;
            }
            
            // Pad with spaces
            size_t len = strlen(temp);
            while (width > (int)len && pos < size - 1) {
                buffer[pos++] = ' ';
                written++;
                width--;
            }
            
            // Copy the string
            for (size_t j = 0; j < len && pos < size - 1; j++) {
                buffer[pos++] = temp[j];
                written++;
            }
        } else {
            buffer[pos++] = *format++;
            written++;
        }
    }
    
    buffer[pos] = '\0';
    va_end(args);
    
    return written;
}

int vsnprintf(char* buffer, size_t size, const char* format, va_list args) {
    // Simplified version - reuse snprintf
    return snprintf(buffer, size, format, args);
}

// -----------------------------------------------------------------------------
// strdup - Duplicate a string
// -----------------------------------------------------------------------------

#include "../include/nebula.h"

char* strdup(const char* str) {
    if (!str) return NULL;
    
    size_t len = strlen(str) + 1;
    char* new_str = (char*)malloc(len);
    if (new_str) {
        memcpy(new_str, str, len);
    }
    return new_str;
}
