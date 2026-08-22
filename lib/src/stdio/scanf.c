// NebulaOS - scanf implementation
// ==================================
//
// Minimal scanf with basic format specifiers

#include "../../lib/include/stdio.h"
#include "../../lib/include/stdarg.h"
#include "../../lib/include/string.h"
#include "../../lib/include/ctype.h"
#include "../../kernel/common/include/vga.h"

static char get_char(void) {
    return 0;
}

static int vsscanf(const char* str, const char* format, void* args) {
    (void)args;
    (void)str;
    int matched = 0;
    
    while (*format) {
        if (*format == '%') {
            format++;
            if (!*format) break;
            
            switch (*format) {
                case 'd': case 'u':
                case 'x': case 'X':
                case 's': case 'c':
                    matched++;
                    break;
                default:
                    break;
            }
        }
        format++;
    }
    
    return matched;
}

int sscanf(const char* str, const char* format, ...) {
    (void)str; (void)format;
    return 0;
}