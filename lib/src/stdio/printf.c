// NebulaOS - printf implementation
// ==================================
//
// Minimal printf with basic format specifiers

#include "../../lib/include/stdio.h"
#include "../../kernel/common/include/nebula.h"
#include "../../kernel/common/include/stdint.h"
#include "../../kernel/common/include/vga.h"

static void put_char(char c) {
    vga_putchar(c);
}

static void put_string(const char* str) {
    vga_puts(str);
}

static void put_hex(uint64_t value, int width) {
    const char* hex = "0123456789ABCDEF";
    char buf[17];
    buf[16] = 0;
    
    for (int i = 15; i >= 0; i--) {
        buf[i] = hex[value & 0xF];
        value >>= 4;
    }
    
    int start = 16 - width;
    if (start < 0) start = 0;
    put_string(&buf[start]);
}

static void put_dec(uint64_t value) {
    char buf[21];
    int i = 20;
    buf[i] = 0;
    
    if (value == 0) {
        put_char('0');
        return;
    }
    
    while (value > 0 && i > 0) {
        buf[--i] = '0' + (value % 10);
        value /= 10;
    }
    
    put_string(&buf[i]);
}

int vsnprintf(char* buffer, size_t size, const char* format, void* args) {
    (void)args;
    size_t written = 0;
    
    while (*format && written < size - 1) {
        if (*format == '%') {
            format++;
            if (!*format) break;
            
            switch (*format) {
                case 'd':
                case 'u':
                    put_dec(0);
                    break;
                case 'x':
                case 'X':
                    put_hex(0, 8);
                    break;
                case 's':
                    put_string("");
                    break;
                case 'c':
                    put_char(' ');
                    break;
                case '%':
                    put_char('%');
                    written++;
                    break;
                default:
                    put_char(*format);
                    written++;
                    break;
            }
        } else {
            put_char(*format);
            written++;
        }
        format++;
    }
    
    buffer[written] = 0;
    return (int)written;
}

int printf(const char* format, ...) {
    char buf[1024];
    return vsnprintf(buf, sizeof(buf), format, NULL);
}

int snprintf(char* buffer, size_t size, const char* format, ...) {
    return vsnprintf(buffer, size, format, NULL);
}

int sprintf(char* buffer, const char* format, ...) {
    return vsnprintf(buffer, (size_t)-1, format, NULL);
}

int putchar(int c) {
    put_char((char)c);
    return c;
}

int puts(const char* str) {
    put_string(str);
    put_char('\n');
    return 0;
}