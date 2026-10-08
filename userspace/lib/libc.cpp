// NebulaOS POSIX-Compatible C Library Implementation
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// See LICENCE for the full license text.

#include "libc.h"
#include "syscalls.h"
#include <cstdarg>

// =============================================================================
// String functions
// =============================================================================
size_t strlen(const char* s) {
    size_t len = 0;
    while (s[len] != '\0') {
        ++len;
    }
    return len;
}

char* strcpy(char* dest, const char* src) {
    char* out = dest;
    while ((*dest++ = *src++) != '\0') {
    }
    return out;
}

char* strncpy(char* dest, const char* src, size_t n) {
    size_t i;
    for (i = 0; i < n && src[i] != '\0'; ++i) {
        dest[i] = src[i];
    }
    for (; i < n; ++i) {
        dest[i] = '\0';
    }
    return dest;
}

char* strcat(char* dest, const char* src) {
    char* out = dest + strlen(dest);
    while ((*out++ = *src++) != '\0') {
    }
    return dest;
}

char* strncat(char* dest, const char* src, size_t n) {
    char* out = dest + strlen(dest);
    size_t i;
    for (i = 0; i < n && src[i] != '\0'; ++i) {
        *out++ = src[i];
    }
    *out = '\0';
    return dest;
}

int strcmp(const char* a, const char* b) {
    while (*a != '\0' && *a == *b) {
        ++a;
        ++b;
    }
    return (int)(unsigned char)*a - (int)(unsigned char)*b;
}

int strncmp(const char* a, const char* b, size_t n) {
    if (n == 0) {
        return 0;
    }
    while (n-- > 0 && *a != '\0' && *a == *b) {
        ++a;
        ++b;
    }
    if (n == 0) {
        return 0;
    }
    return (int)(unsigned char)*a - (int)(unsigned char)*b;
}

int strcoll(const char* a, const char* b) {
    return strcmp(a, b);
}

char* strchr(const char* s, int c) {
    while (*s != '\0') {
        if (*s == (char)c) {
            return (char*)s;
        }
        ++s;
    }
    if ((char)c == '\0') {
        return (char*)s;
    }
    return nullptr;
}

char* strrchr(const char* s, int c) {
    const char* last = nullptr;
    while (*s != '\0') {
        if (*s == (char)c) {
            last = s;
        }
        ++s;
    }
    if ((char)c == '\0') {
        return (char*)s;
    }
    return (char*)last;
}

size_t strspn(const char* s, const char* accept) {
    size_t count = 0;
    while (*s != '\0') {
        bool found = false;
        for (const char* a = accept; *a != '\0'; ++a) {
            if (*s == *a) {
                found = true;
                break;
            }
        }
        if (!found) {
            break;
        }
        ++count;
        ++s;
    }
    return count;
}

size_t strcspn(const char* s, const char* reject) {
    size_t count = 0;
    while (*s != '\0') {
        bool found = false;
        for (const char* r = reject; *r != '\0'; ++r) {
            if (*s == *r) {
                found = true;
                break;
            }
        }
        if (found) {
            break;
        }
        ++count;
        ++s;
    }
    return count;
}

char* strpbrk(const char* s, const char* accept) {
    while (*s != '\0') {
        for (const char* a = accept; *a != '\0'; ++a) {
            if (*s == *a) {
                return (char*)s;
            }
        }
        ++s;
    }
    return nullptr;
}

char* strstr(const char* haystack, const char* needle) {
    if (*needle == '\0') {
        return (char*)haystack;
    }
    while (*haystack != '\0') {
        const char* h = haystack;
        const char* n = needle;
        while (*n != '\0' && *h == *n) {
            ++h;
            ++n;
        }
        if (*n == '\0') {
            return (char*)haystack;
        }
        ++haystack;
    }
    return nullptr;
}

char* strtok(char* s, const char* delim) {
    static char* saved = nullptr;
    if (s == nullptr) {
        s = saved;
    }
    if (s == nullptr) {
        return nullptr;
    }

    // Skip leading delimiters
    while (*s != '\0' && strchr(delim, *s) != nullptr) {
        ++s;
    }

    if (*s == '\0') {
        saved = nullptr;
        return nullptr;
    }

    char* token = s;
    while (*s != '\0' && strchr(delim, *s) == nullptr) {
        ++s;
    }

    if (*s == '\0') {
        saved = nullptr;
    } else {
        *s = '\0';
        saved = s + 1;
    }

    return token;
}

char* strdup(const char* s) {
    if (s == nullptr) {
        return nullptr;
    }
    const size_t len = strlen(s) + 1;
    char* copy = (char*)malloc(len);
    if (copy == nullptr) {
        return nullptr;
    }
    memcpy(copy, s, len);
    return copy;
}

char* strndup(const char* s, size_t n) {
    if (s == nullptr) {
        return nullptr;
    }
    size_t len = strlen(s);
    if (len > n) {
        len = n;
    }
    char* copy = (char*)malloc(len + 1);
    if (copy == nullptr) {
        return nullptr;
    }
    memcpy(copy, s, len);
    copy[len] = '\0';
    return copy;
}

size_t strxfrm(char* dest, const char* src, size_t n) {
    size_t len = strlen(src);
    if (n > 0) {
        size_t copy = len < n ? len : n - 1;
        memcpy(dest, src, copy);
        dest[copy] = '\0';
    }
    return len;
}

void* memset(void* s, int c, size_t n) {
    unsigned char* p = (unsigned char*)s;
    for (size_t i = 0; i < n; ++i) {
        p[i] = (unsigned char)c;
    }
    return s;
}

void* memcpy(void* dest, const void* src, size_t n) {
    unsigned char* d = (unsigned char*)dest;
    const unsigned char* s = (const unsigned char*)src;
    for (size_t i = 0; i < n; ++i) {
        d[i] = s[i];
    }
    return dest;
}

void* memmove(void* dest, const void* src, size_t n) {
    unsigned char* d = (unsigned char*)dest;
    const unsigned char* s = (const unsigned char*)src;

    if (d == s) {
        return dest;
    }

    if (d < s) {
        for (size_t i = 0; i < n; ++i) {
            d[i] = s[i];
        }
    } else {
        for (size_t i = n; i > 0; --i) {
            d[i - 1] = s[i - 1];
        }
    }

    return dest;
}

int memcmp(const void* a, const void* b, size_t n) {
    const unsigned char* pa = (const unsigned char*)a;
    const unsigned char* pb = (const unsigned char*)b;
    for (size_t i = 0; i < n; ++i) {
        if (pa[i] != pb[i]) {
            return (int)pa[i] - (int)pb[i];
        }
    }
    return 0;
}

void* memchr(const void* s, int c, size_t n) {
    const unsigned char* p = (const unsigned char*)s;
    for (size_t i = 0; i < n; ++i) {
        if (p[i] == (unsigned char)c) {
            return (void*)(p + i);
        }
    }
    return nullptr;
}

void* memrchr(const void* s, int c, size_t n) {
    const unsigned char* p = (const unsigned char*)s;
    for (size_t i = n; i > 0; --i) {
        if (p[i - 1] == (unsigned char)c) {
            return (void*)(p + i - 1);
        }
    }
    return nullptr;
}

// =============================================================================
// stdio
// =============================================================================
static FILE std_files[3] = {
    { 0, nullptr, 0, 0, _IONBF },  // stdin
    { 1, nullptr, 0, 0, _IONBF },  // stdout
    { 2, nullptr, 0, 0, _IONBF }   // stderr
};

FILE* stdin = &std_files[0];
FILE* stdout = &std_files[1];
FILE* stderr = &std_files[2];

static FILE file_pool[FOPEN_MAX - 3];

static int format_char(char* buf, size_t size, const char*& fmt,
                        va_list ap, int& written) {
    (void)buf;
    (void)size;
    (void)written;

    char c = *fmt;
    if (c == '%') {
        ++fmt;
        // Parse flags
        bool left_align = false;
        bool plus = false;
        bool space = false;
        bool zero_pad = false;
        bool alt = false;

        while (true) {
            switch (*fmt) {
                case '-': left_align = true; ++fmt; break;
                case '+': plus = true; ++fmt; break;
                case ' ': space = true; ++fmt; break;
                case '0': zero_pad = true; ++fmt; break;
                case '#': alt = true; ++fmt; break;
                default: goto flags_done;
            }
        }
        flags_done:

        // Parse width
        int width = 0;
        while (*fmt >= '0' && *fmt <= '9') {
            width = width * 10 + (*fmt - '0');
            ++fmt;
        }

        // Parse precision
        int precision = -1;
        if (*fmt == '.') {
            ++fmt;
            precision = 0;
            while (*fmt >= '0' && *fmt <= '9') {
                precision = precision * 10 + (*fmt - '0');
                ++fmt;
            }
        }

        // Parse length modifier
        int length = 0;  // 0=none, 1=hh, 2=h, 3=l, 4=ll
        if (*fmt == 'h') {
            ++fmt;
            if (*fmt == 'h') {
                length = 1;
                ++fmt;
            } else {
                length = 2;
            }
        } else if (*fmt == 'l') {
            ++fmt;
            if (*fmt == 'l') {
                length = 4;
                ++fmt;
            } else {
                length = 3;
            }
        }

        // Parse conversion specifier
        char spec = *fmt;
        if (spec == '\0') {
            return -1;
        }
        ++fmt;

        (void)left_align;
        (void)plus;
        (void)space;
        (void)zero_pad;
        (void)alt;
        (void)width;
        (void)precision;
        (void)length;

        return (int)spec;
    }

    ++fmt;
    return (int)c;
}

int vsnprintf(char* out, size_t size, const char* fmt, va_list ap) {
    if (size == 0) {
        return 0;
    }

    size_t pos = 0;
    const char* p = fmt;

    while (*p != '\0') {
        if (*p != '%') {
            if (pos < size - 1) {
                out[pos] = *p;
            }
            ++pos;
            ++p;
            continue;
        }

        ++p;

        // Parse flags
        bool left_align = false;
        bool plus = false;
        bool space = false;
        bool zero_pad = false;
        bool alt = false;

        while (true) {
            switch (*p) {
                case '-': left_align = true; ++p; break;
                case '+': plus = true; ++p; break;
                case ' ': space = true; ++p; break;
                case '0': zero_pad = true; ++p; break;
                case '#': alt = true; ++p; break;
                default: goto flags_done;
            }
        }
        flags_done:

        // Parse width
        int width = 0;
        while (*p >= '0' && *p <= '9') {
            width = width * 10 + (*p - '0');
            ++p;
        }

        // Parse precision
        int precision = -1;
        if (*p == '.') {
            ++p;
            precision = 0;
            while (*p >= '0' && *p <= '9') {
                precision = precision * 10 + (*p - '0');
                ++p;
            }
        }

        // Parse length modifier
        int length = 0;
        if (*p == 'h') {
            ++p;
            if (*p == 'h') {
                length = 1;
                ++p;
            } else {
                length = 2;
            }
        } else if (*p == 'l') {
            ++p;
            if (*p == 'l') {
                length = 4;
                ++p;
            } else {
                length = 3;
            }
        }

        char spec = *p;
        if (spec == '\0') {
            break;
        }
        ++p;

        // Handle conversion
        char num_buf[32];
        char str_buf[512];
        const char* str = nullptr;
        int num_len = 0;
        int pad_len = 0;

        switch (spec) {
            case 'd':
            case 'i': {
                long val = 0;
                switch (length) {
                    case 1: val = (signed char)va_arg(ap, int); break;
                    case 2: val = (short)va_arg(ap, int); break;
                    case 3: val = va_arg(ap, long); break;
                    case 4: val = va_arg(ap, long long); break;
                    default: val = va_arg(ap, int); break;
                }

                char* num = num_buf + sizeof(num_buf);
                *--num = '\0';
                bool negative = val < 0;
                unsigned long uval = negative ? (unsigned long)(-val) : (unsigned long)val;

                do {
                    *--num = '0' + (uval % 10);
                    uval /= 10;
                } while (uval != 0);

                if (negative) {
                    *--num = '-';
                } else if (plus) {
                    *--num = '+';
                } else if (space) {
                    *--num = ' ';
                }

                str = num;
                num_len = (int)strlen(num);
                break;
            }

            case 'u': {
                unsigned long val = 0;
                switch (length) {
                    case 1: val = (unsigned char)va_arg(ap, unsigned int); break;
                    case 2: val = (unsigned short)va_arg(ap, unsigned int); break;
                    case 3: val = va_arg(ap, unsigned long); break;
                    case 4: val = va_arg(ap, unsigned long long); break;
                    default: val = va_arg(ap, unsigned int); break;
                }

                char* num = num_buf + sizeof(num_buf);
                *--num = '\0';
                do {
                    *--num = '0' + (val % 10);
                    val /= 10;
                } while (val != 0);

                str = num;
                num_len = (int)strlen(num);
                break;
            }

            case 'x':
            case 'X': {
                unsigned long val = 0;
                switch (length) {
                    case 1: val = (unsigned char)va_arg(ap, unsigned int); break;
                    case 2: val = (unsigned short)va_arg(ap, unsigned int); break;
                    case 3: val = va_arg(ap, unsigned long); break;
                    case 4: val = va_arg(ap, unsigned long long); break;
                    default: val = va_arg(ap, unsigned int); break;
                }

                char* num = num_buf + sizeof(num_buf);
                *--num = '\0';
                const char* hex = (spec == 'x') ? "0123456789abcdef" : "0123456789ABCDEF";
                do {
                    *--num = hex[val & 0xF];
                    val >>= 4;
                } while (val != 0);

                if (alt && num[0] != '0') {
                    *--num = (spec == 'x') ? 'x' : 'X';
                    *--num = '0';
                }

                str = num;
                num_len = (int)strlen(num);
                break;
            }

            case 'o': {
                unsigned long val = 0;
                switch (length) {
                    case 1: val = (unsigned char)va_arg(ap, unsigned int); break;
                    case 2: val = (unsigned short)va_arg(ap, unsigned int); break;
                    case 3: val = va_arg(ap, unsigned long); break;
                    case 4: val = va_arg(ap, unsigned long long); break;
                    default: val = va_arg(ap, unsigned int); break;
                }

                char* num = num_buf + sizeof(num_buf);
                *--num = '\0';
                do {
                    *--num = '0' + (val & 7);
                    val >>= 3;
                } while (val != 0);

                str = num;
                num_len = (int)strlen(num);
                break;
            }

            case 'p': {
                unsigned long val = (unsigned long)va_arg(ap, void*);
                char* num = num_buf + sizeof(num_buf);
                *--num = '\0';
                do {
                    *--num = "0123456789abcdef"[val & 0xF];
                    val >>= 4;
                } while (val != 0);
                *--num = 'x';
                *--num = '0';
                str = num;
                num_len = (int)strlen(num);
                break;
            }

            case 'c': {
                str_buf[0] = (char)va_arg(ap, int);
                str_buf[1] = '\0';
                str = str_buf;
                num_len = 1;
                break;
            }

            case 's': {
                str = va_arg(ap, const char*);
                if (str == nullptr) {
                    str = "(null)";
                }
                num_len = (int)strlen(str);
                if (precision >= 0 && num_len > precision) {
                    num_len = precision;
                }
                break;
            }

            case '%': {
                str_buf[0] = '%';
                str_buf[1] = '\0';
                str = str_buf;
                num_len = 1;
                break;
            }

            default:
                str_buf[0] = '%';
                str_buf[1] = spec;
                str_buf[2] = '\0';
                str = str_buf;
                num_len = 2;
                break;
        }

        // Calculate padding
        pad_len = width - num_len;
        if (pad_len < 0) {
            pad_len = 0;
        }

        // Write padding (before, unless left-aligned)
        if (!left_align) {
            char pad_char = zero_pad ? '0' : ' ';
            for (int i = 0; i < pad_len; ++i) {
                if (pos < size - 1) {
                    out[pos] = pad_char;
                }
                ++pos;
            }
        }

        // Write the string
        for (int i = 0; i < num_len; ++i) {
            if (pos < size - 1) {
                out[pos] = str[i];
            }
            ++pos;
        }

        // Write padding (after, if left-aligned)
        if (left_align) {
            for (int i = 0; i < pad_len; ++i) {
                if (pos < size - 1) {
                    out[pos] = ' ';
                }
                ++pos;
            }
        }
    }

    out[pos < size ? pos : size - 1] = '\0';
    return (int)pos;
}

int vsprintf(char* out, const char* fmt, va_list ap) {
    return vsnprintf(out, (size_t)-1, fmt, ap);
}

int snprintf(char* out, size_t size, const char* fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    int result = vsnprintf(out, size, fmt, ap);
    va_end(ap);
    return result;
}

int sprintf(char* out, const char* fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    int result = vsnprintf(out, (size_t)-1, fmt, ap);
    va_end(ap);
    return result;
}

int vprintf(const char* fmt, va_list ap) {
    char buf[1024];
    int result = vsnprintf(buf, sizeof(buf), fmt, ap);
    if (result > 0) {
        write(1, buf, result);
    }
    return result;
}

int vfprintf(FILE* stream, const char* fmt, va_list ap) {
    if (stream == nullptr) {
        return -1;
    }
    char buf[1024];
    int result = vsnprintf(buf, sizeof(buf), fmt, ap);
    if (result > 0) {
        write(stream->fd, buf, result);
    }
    return result;
}

int printf(const char* fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    int result = vprintf(fmt, ap);
    va_end(ap);
    return result;
}

int fprintf(FILE* stream, const char* fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    int result = vfprintf(stream, fmt, ap);
    va_end(ap);
    return result;
}

int puts(const char* s) {
    if (s == nullptr) {
        return EOF;
    }
    size_t len = strlen(s);
    write(1, s, len);
    write(1, "\n", 1);
    return len + 1;
}

int fputs(const char* s, FILE* stream) {
    if (s == nullptr || stream == nullptr) {
        return EOF;
    }
    size_t len = strlen(s);
    write(stream->fd, s, len);
    return len;
}

int putchar(int c) {
    char ch = (char)c;
    write(1, &ch, 1);
    return (unsigned char)c;
}

int fputc(int c, FILE* stream) {
    if (stream == nullptr) {
        return EOF;
    }
    char ch = (char)c;
    write(stream->fd, &ch, 1);
    return (unsigned char)c;
}

int getchar(void) {
    char c;
    ssize_t result = read(0, &c, 1);
    if (result <= 0) {
        return EOF;
    }
    return (unsigned char)c;
}

int fgetc(FILE* stream) {
    if (stream == nullptr) {
        return EOF;
    }
    char c;
    ssize_t result = read(stream->fd, &c, 1);
    if (result <= 0) {
        return EOF;
    }
    return (unsigned char)c;
}

int fgets(char* buf, int n, FILE* stream) {
    if (buf == nullptr || stream == nullptr || n <= 0) {
        return nullptr;
    }

    int i;
    for (i = 0; i < n - 1; ++i) {
        char c;
        ssize_t result = read(stream->fd, &c, 1);
        if (result <= 0) {
            break;
        }
        buf[i] = c;
        if (c == '\n') {
            ++i;
            break;
        }
    }

    if (i == 0) {
        return nullptr;
    }

    buf[i] = '\0';
    return buf;
}

int ungetc(int c, FILE* stream) {
    (void)stream;
    if (c == EOF) {
        return EOF;
    }
    return c;
}

int fflush(FILE* stream) {
    (void)stream;
    return 0;
}

FILE* fopen(const char* path, const char* mode) {
    if (path == nullptr || mode == nullptr) {
        return nullptr;
    }

    int flags = 0;
    bool reading = false;
    bool writing = false;
    bool append = false;

    for (const char* m = mode; *m != '\0'; ++m) {
        switch (*m) {
            case 'r': reading = true; break;
            case 'w': writing = true; break;
            case 'a': append = true; break;
            case '+': reading = true; writing = true; break;
            default: break;
        }
    }

    if (writing) {
        flags = O_WRONLY | O_CREAT | O_TRUNC;
    } else if (append) {
        flags = O_WRONLY | O_CREAT | O_APPEND;
    } else if (reading) {
        flags = O_RDONLY;
    }

    if (reading && writing) {
        flags = O_RDWR;
    }

    int fd = open(path, flags, 0644);
    if (fd < 0) {
        return nullptr;
    }

    // Find a free slot in the file pool
    for (auto& f : file_pool) {
        if (f.fd == 0 && f.buffer == nullptr) {
            f.fd = fd;
            f.buffer = nullptr;
            f.buffer_size = 0;
            f.buffer_pos = 0;
            f.flags = _IONBF;
            return &f;
        }
    }

    close(fd);
    return nullptr;
}

int fclose(FILE* stream) {
    if (stream == nullptr) {
        return EOF;
    }
    if (stream->fd >= 0) {
        close(stream->fd);
    }
    stream->fd = 0;
    stream->buffer = nullptr;
    stream->buffer_size = 0;
    stream->buffer_pos = 0;
    return 0;
}

size_t fread(void* ptr, size_t size, size_t n, FILE* stream) {
    if (ptr == nullptr || stream == nullptr || size == 0 || n == 0) {
        return 0;
    }
    ssize_t result = read(stream->fd, ptr, size * n);
    if (result < 0) {
        return 0;
    }
    return result / size;
}

size_t fwrite(const void* ptr, size_t size, size_t n, FILE* stream) {
    if (ptr == nullptr || stream == nullptr || size == 0 || n == 0) {
        return 0;
    }
    ssize_t result = write(stream->fd, ptr, size * n);
    if (result < 0) {
        return 0;
    }
    return result / size;
}

int fseek(FILE* stream, long offset, int whence) {
    if (stream == nullptr) {
        return -1;
    }
    off_t result = lseek(stream->fd, offset, whence);
    return result < 0 ? -1 : 0;
}

long ftell(FILE* stream) {
    if (stream == nullptr) {
        return -1;
    }
    return lseek(stream->fd, 0, SEEK_CUR);
}

void rewind(FILE* stream) {
    if (stream != nullptr) {
        fseek(stream, 0, SEEK_SET);
    }
}

int feof(FILE* stream) {
    if (stream == nullptr) {
        return 0;
    }
    return (stream->flags & _IOEOF) != 0;
}

int ferror(FILE* stream) {
    if (stream == nullptr) {
        return 0;
    }
    return (stream->flags & _IOERR) != 0;
}

void clearerr(FILE* stream) {
    if (stream != nullptr) {
        stream->flags &= ~(_IOEOF | _IOERR);
    }
}

void perror(const char* s) {
    if (s != nullptr) {
        fputs(s, stderr);
        fputs(": ", stderr);
    }
    // In a real implementation, we'd look up the error string
    fputs("error\n", stderr);
}

// =============================================================================
// stdlib
// =============================================================================
int atoi(const char* s) {
    if (s == nullptr) {
        return 0;
    }
    while (*s == ' ' || (*s >= '\t' && *s <= '\r')) {
        ++s;
    }
    bool negative = false;
    if (*s == '-') {
        negative = true;
        ++s;
    } else if (*s == '+') {
        ++s;
    }
    int result = 0;
    while (*s >= '0' && *s <= '9') {
        result = result * 10 + (*s - '0');
        ++s;
    }
    return negative ? -result : result;
}

long atol(const char* s) {
    if (s == nullptr) {
        return 0;
    }
    while (*s == ' ' || (*s >= '\t' && *s <= '\r')) {
        ++s;
    }
    bool negative = false;
    if (*s == '-') {
        negative = true;
        ++s;
    } else if (*s == '+') {
        ++s;
    }
    long result = 0;
    while (*s >= '0' && *s <= '9') {
        result = result * 10 + (*s - '0');
        ++s;
    }
    return negative ? -result : result;
}

long long atoll(const char* s) {
    if (s == nullptr) {
        return 0;
    }
    while (*s == ' ' || (*s >= '\t' && *s <= '\r')) {
        ++s;
    }
    bool negative = false;
    if (*s == '-') {
        negative = true;
        ++s;
    } else if (*s == '+') {
        ++s;
    }
    long long result = 0;
    while (*s >= '0' && *s <= '9') {
        result = result * 10 + (*s - '0');
        ++s;
    }
    return negative ? -result : result;
}

double atof(const char* s) {
    if (s == nullptr) {
        return 0.0;
    }
    while (*s == ' ' || (*s >= '\t' && *s <= '\r')) {
        ++s;
    }
    bool negative = false;
    if (*s == '-') {
        negative = true;
        ++s;
    } else if (*s == '+') {
        ++s;
    }
    double result = 0.0;
    while (*s >= '0' && *s <= '9') {
        result = result * 10.0 + (*s - '0');
        ++s;
    }
    if (*s == '.') {
        ++s;
        double fraction = 0.1;
        while (*s >= '0' && *s <= '9') {
            result += (*s - '0') * fraction;
            fraction *= 0.1;
            ++s;
        }
    }
    return negative ? -result : result;
}

// Simple bump allocator for kernel/userspace
static char heap_memory[1024 * 1024];
static size_t heap_used = 0;

void* malloc(size_t size) {
    if (size == 0) {
        return nullptr;
    }
    // Align to 16 bytes
    size = (size + 15) & ~((size_t)15);
    if (heap_used + size > sizeof(heap_memory)) {
        return nullptr;
    }
    void* ptr = heap_memory + heap_used;
    heap_used += size;
    return ptr;
}

void free(void* ptr) {
    (void)ptr;
    // Bump allocator doesn't free
}

void* calloc(size_t count, size_t size) {
    if (count == 0 || size == 0) {
        return nullptr;
    }
    size_t total = count * size;
    void* ptr = malloc(total);
    if (ptr != nullptr) {
        memset(ptr, 0, total);
    }
    return ptr;
}

void* realloc(void* ptr, size_t size) {
    if (ptr == nullptr) {
        return malloc(size);
    }
    if (size == 0) {
        free(ptr);
        return nullptr;
    }
    // For bump allocator, just allocate new and copy
    // (we don't know the old size, so we assume it's large enough)
    void* new_ptr = malloc(size);
    if (new_ptr != nullptr) {
        memcpy(new_ptr, ptr, size);
    }
    return new_ptr;
}

void abort(void) {
    // Raise SIGABRT
    // In a real implementation, we'd send a signal
    for (;;) {
        asm volatile("hlt");
    }
}

void exit(int status) {
    (void)status;
    // Exit the process
    _exit(status);
    for (;;) {
        asm volatile("hlt");
    }
}

void _exit(int status) {
    (void)status;
    syscall_exit(0);
    for (;;) {
        asm volatile("hlt");
    }
}

int atexit(void (*func)(void)) {
    (void)func;
    return 0;
}

int abs(int n) {
    return n < 0 ? -n : n;
}

long labs(long n) {
    return n < 0 ? -n : n;
}

long long llabs(long long n) {
    return n < 0 ? -n : n;
}

int rand(void) {
    static unsigned int seed = 1;
    seed = seed * 1103515245 + 12345;
    return (int)((seed >> 16) & 0x7FFF);
}

void srand(unsigned int seed) {
    // In a real implementation, we'd set the global seed
    (void)seed;
}

int system(const char* command) {
    (void)command;
    return -1;
}

char* getenv(const char* name) {
    (void)name;
    return nullptr;
}

int setenv(const char* name, const char* value, int overwrite) {
    (void)name;
    (void)value;
    (void)overwrite;
    return 0;
}

int unsetenv(const char* name) {
    (void)name;
    return 0;
}

int posix_memalign(void** memptr, size_t alignment, size_t size) {
    if (memptr == nullptr) {
        return EINVAL;
    }
    if (alignment < sizeof(void*) || (alignment & (alignment - 1)) != 0) {
        return EINVAL;
    }
    void* ptr = malloc(size + alignment);
    if (ptr == nullptr) {
        return ENOMEM;
    }
    void* aligned = (void*)(((uintptr_t)ptr + alignment) & ~(alignment - 1));
    *memptr = aligned;
    return 0;
}

void* aligned_alloc(size_t alignment, size_t size) {
    void* ptr = nullptr;
    if (posix_memalign(&ptr, alignment, size) != 0) {
        return nullptr;
    }
    return ptr;
}

// =============================================================================
// unistd
// =============================================================================
int open(const char* path, int flags, ...) {
    if (path == nullptr) {
        return -1;
    }
    mode_t mode = 0;
    // Handle optional mode argument
    va_list ap;
    va_start(ap, flags);
    mode = va_arg(ap, mode_t);
    va_end(ap);

    return syscall_open(path, flags, mode);
}

int close(int fd) {
    return syscall_close(fd);
}

ssize_t read(int fd, void* buf, size_t count) {
    return syscall_read(fd, buf, count);
}

ssize_t write(int fd, const void* buf, size_t count) {
    return syscall_write(fd, buf, count);
}

off_t lseek(int fd, off_t offset, int whence) {
    return syscall_lseek(fd, offset, whence);
}

int fstat(int fd, void* stat) {
    return syscall_fstat(fd, stat);
}

int stat(const char* path, void* stat) {
    return syscall_stat(path, stat);
}

int lstat(const char* path, void* stat) {
    return syscall_lstat(path, stat);
}

int access(const char* path, int mode) {
    return syscall_access(path, mode);
}

int dup(int fd) {
    return syscall_dup(fd);
}

int dup2(int oldfd, int newfd) {
    return syscall_dup2(oldfd, newfd);
}

int pipe(int fds[2]) {
    return syscall_pipe(fds);
}

int isatty(int fd) {
    return syscall_isatty(fd);
}

int truncate(const char* path, off_t length) {
    return syscall_truncate(path, length);
}

int ftruncate(int fd, off_t length) {
    return syscall_ftruncate(fd, length);
}

ssize_t readlink(const char* path, char* buf, size_t bufsiz) {
    return syscall_readlink(path, buf, bufsiz);
}

int symlink(const char* target, const char* linkpath) {
    return syscall_symlink(target, linkpath);
}

int link(const char* oldpath, const char* newpath) {
    return syscall_link(oldpath, newpath);
}

int unlink(const char* path) {
    return syscall_unlink(path);
}

int rmdir(const char* path) {
    return syscall_rmdir(path);
}

int mkdir(const char* path, mode_t mode) {
    return syscall_mkdir(path, mode);
}

int chdir(const char* path) {
    return syscall_chdir(path);
}

int fchdir(int fd) {
    return syscall_fchdir(fd);
}

char* getcwd(char* buf, size_t size) {
    return syscall_getcwd(buf, size);
}

int chroot(const char* path) {
    return syscall_chroot(path);
}

int fsync(int fd) {
    return syscall_fsync(fd);
}

int fdatasync(int fd) {
    return syscall_fdatasync(fd);
}

int fcntl(int fd, int cmd, ...) {
    (void)fd;
    (void)cmd;
    return -1;
}

int ioctl(int fd, unsigned long request, ...) {
    (void)fd;
    (void)request;
    return -1;
}

int swapon(const char* path, int swap_flags) {
    (void)path;
    (void)swap_flags;
    return -1;
}

int swapoff(const char* path) {
    (void)path;
    return -1;
}

int reboot(int magic, int magic2, unsigned int cmd, void* arg) {
    (void)magic;
    (void)magic2;
    (void)cmd;
    (void)arg;
    return -1;
}

int gethostname(char* name, size_t len) {
    (void)name;
    (void)len;
    return -1;
}

int sethostname(const char* name, size_t len) {
    (void)name;
    (void)len;
    return -1;
}

int getpid(void) {
    return syscall_getpid();
}

int getppid(void) {
    return syscall_getppid();
}

int getuid(void) {
    return 0;
}

int geteuid(void) {
    return 0;
}

int getgid(void) {
    return 0;
}

int getegid(void) {
    return 0;
}

int setuid(uid_t uid) {
    (void)uid;
    return 0;
}

int setgid(gid_t gid) {
    (void)gid;
    return 0;
}

int setpgid(pid_t pid, pid_t pgid) {
    (void)pid;
    (void)pgid;
    return -1;
}

pid_t getpgid(pid_t pid) {
    (void)pid;
    return 0;
}

pid_t setsid(void) {
    return 0;
}

pid_t getsid(pid_t pid) {
    (void)pid;
    return 0;
}

int usleep(useconds_t usec) {
    // Convert microseconds to milliseconds
    unsigned int ms = usec / 1000;
    return sleep(ms);
}

int sleep(unsigned int seconds) {
    // Convert seconds to ticks
    unsigned int ticks = seconds * 100;  // Assuming 100 Hz
    return syscall_nanosleep(ticks);
}

int nanosleep(const struct timespec* req, struct timespec* rem) {
    (void)rem;
    if (req == nullptr) {
        return -1;
    }
    unsigned int ms = (unsigned int)(req->tv_sec * 1000 + req->tv_nsec / 1000000);
    return sleep(ms);
}

// =============================================================================
// sys/stat
// =============================================================================
// (Already defined in header)

// =============================================================================
// dirent
// =============================================================================
DIR* opendir(const char* name) {
    (void)name;
    return nullptr;
}

struct dirent* readdir(DIR* dirp) {
    (void)dirp;
    return nullptr;
}

void rewinddir(DIR* dirp) {
    (void)dirp;
}

int closedir(DIR* dirp) {
    (void)dirp;
    return -1;
}

int dirfd(DIR* dirp) {
    (void)dirp;
    return -1;
}

// =============================================================================
// time
// =============================================================================
time_t time(time_t* t) {
    time_t result = (time_t)syscall_time();
    if (t != nullptr) {
        *t = result;
    }
    return result;
}

int gettimeofday(struct timeval* tv, void* tz) {
    (void)tz;
    if (tv == nullptr) {
        return -1;
    }
    time_t now = time(nullptr);
    tv->tv_sec = now;
    tv->tv_usec = 0;
    return 0;
}

clock_t clock(void) {
    return (clock_t)syscall_ticks();
}

double difftime(time_t t1, time_t t0) {
    return (double)(t1 - t0);
}

time_t mktime(struct tm* tm) {
    (void)tm;
    return (time_t)-1;
}

char* asctime(const struct tm* tm) {
    (void)tm;
    return nullptr;
}

char* ctime(const time_t* timep) {
    (void)timep;
    return nullptr;
}

struct tm* gmtime(const time_t* timep) {
    (void)timep;
    return nullptr;
}

struct tm* localtime(const time_t* timep) {
    (void)timep;
    return nullptr;
}

size_t strftime(char* s, size_t max, const char* fmt, const struct tm* tm) {
    (void)s;
    (void)max;
    (void)fmt;
    (void)tm;
    return 0;
}

// =============================================================================
// Signal handling
// =============================================================================
sighandler_t signal(int signum, sighandler_t handler) {
    (void)signum;
    (void)handler;
    return nullptr;
}

int raise(int signum) {
    (void)signum;
    return 0;
}

int kill(pid_t pid, int signum) {
    (void)pid;
    (void)signum;
    return -1;
}

// =============================================================================
// setjmp/longjmp
// =============================================================================
// These are typically implemented in assembly, but we provide
// a simple C implementation for now

int setjmp(jmp_buf env) {
    asm volatile(
        "mov %%ebx, %0\n"
        "mov %%ecx, %1\n"
        "mov %%edx, %2\n"
        "mov %%esi, %3\n"
        "mov %%edi, %4\n"
        "mov %%ebp, %5\n"
        "mov %%esp, %6\n"
        "mov %%eip, %7\n"
        : "=m"(env[0].ebx), "=m"(env[0].ecx),
          "=m"(env[0].edx), "=m"(env[0].esi),
          "=m"(env[0].edi), "=m"(env[0].ebp),
          "=m"(env[0].esp), "=m"(env[0].eip)
        :
        : "memory"
    );
    return 0;
}

void longjmp(jmp_buf env, int val) {
    asm volatile(
        "mov %0, %%ebx\n"
        "mov %1, %%ecx\n"
        "mov %2, %%edx\n"
        "mov %3, %%esi\n"
        "mov %4, %%edi\n"
        "mov %5, %%ebp\n"
        "mov %6, %%esp\n"
        "mov %7, %%eip\n"
        :
        : "m"(env[0].ebx), "m"(env[0].ecx),
          "m"(env[0].edx), "m"(env[0].esi),
          "m"(env[0].edi), "m"(env[0].ebp),
          "m"(env[0].esp), "m"(env[0].eip)
        : "memory"
    );
    (void)val;
    // Never returns
    for (;;) {
        asm volatile("hlt");
    }
}

// =============================================================================
// pthread (minimal)
// =============================================================================
int pthread_create(pthread_t* thread, void* attr,
                   void* (*start)(void*), void* arg) {
    (void)thread;
    (void)attr;
    (void)start;
    (void)arg;
    return -1;
}

int pthread_join(pthread_t thread, void** retval) {
    (void)thread;
    (void)retval;
    return -1;
}

int pthread_detach(pthread_t thread) {
    (void)thread;
    return -1;
}

int pthread_exit(void* retval) {
    (void)retval;
    for (;;) {
        asm volatile("hlt");
    }
}

int pthread_mutex_init(pthread_mutex_t* mutex, void* attr) {
    (void)attr;
    if (mutex == nullptr) {
        return -1;
    }
    mutex->locked = 0;
    return 0;
}

int pthread_mutex_destroy(pthread_mutex_t* mutex) {
    if (mutex == nullptr) {
        return -1;
    }
    mutex->locked = 0;
    return 0;
}

int pthread_mutex_lock(pthread_mutex_t* mutex) {
    if (mutex == nullptr) {
        return -1;
    }
    while (__sync_lock_test_and_set(&mutex->locked, 1)) {
        // Spin
    }
    return 0;
}

int pthread_mutex_unlock(pthread_mutex_t* mutex) {
    if (mutex == nullptr) {
        return -1;
    }
    __sync_lock_release(&mutex->locked);
    return 0;
}

// =============================================================================
// framebuffer device
// =============================================================================
int open_framebuffer(framebuffer_info_t* info) {
    if (info == nullptr) {
        return -1;
    }
    return syscall_get_framebuffer_info(0, info);
}

void close_framebuffer(int fd) {
    (void)fd;
}

unsigned int fb_pixel(unsigned int x, unsigned int y) {
    framebuffer_info_t info;
    if (open_framebuffer(&info) <= 0) {
        return 0;
    }
    if (x >= info.width || y >= info.height) {
        return 0;
    }
    unsigned int* buffer = (unsigned int*)info.buffer;
    return buffer[y * info.pitch / 4 + x];
}

void fb_put_pixel(unsigned int x, unsigned int y, unsigned int color) {
    framebuffer_info_t info;
    if (open_framebuffer(&info) <= 0) {
        return;
    }
    if (x >= info.width || y >= info.height) {
        return;
    }
    unsigned int* buffer = (unsigned int*)info.buffer;
    buffer[y * info.pitch / 4 + x] = color;
}

void fb_fill_rect(unsigned int x, unsigned int y, unsigned int w, unsigned int h, unsigned int color) {
    framebuffer_info_t info;
    if (open_framebuffer(&info) <= 0) {
        return;
    }
    unsigned int* buffer = (unsigned int*)info.buffer;
    for (unsigned int row = 0; row < h; ++row) {
        for (unsigned int col = 0; col < w; ++col) {
            unsigned int px = x + col;
            unsigned int py = y + row;
            if (px < info.width && py < info.height) {
                buffer[py * info.pitch / 4 + px] = color;
            }
        }
    }
}

void fb_present(void) {
    syscall_present();
}

unsigned int fb_width(void) {
    framebuffer_info_t info;
    if (open_framebuffer(&info) <= 0) {
        return 0;
    }
    return info.width;
}

unsigned int fb_height(void) {
    framebuffer_info_t info;
    if (open_framebuffer(&info) <= 0) {
        return 0;
    }
    return info.height;
}

// =============================================================================
// Misc
// =============================================================================
void* memset_bytes(void* s, int c, size_t n) {
    return memset(s, c, n);
}

int isspace(int c) {
    return c == ' ' || (c >= '\t' && c <= '\r');
}

int isdigit(int c) {
    return c >= '0' && c <= '9';
}

int isalpha(int c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z');
}

int isalnum(int c) {
    return isalpha(c) || isdigit(c);
}

int isupper(int c) {
    return c >= 'A' && c <= 'Z';
}

int islower(int c) {
    return c >= 'a' && c <= 'z';
}

int ispunct(int c) {
    return isgraph(c) && !isalnum(c);
}

int isgraph(int c) {
    return c > ' ' && c < 0x7F;
}

int isprint(int c) {
    return c >= ' ' && c < 0x7F;
}

int iscntrl(int c) {
    return (c >= 0 && c < ' ') || c == 0x7F;
}

int isxdigit(int c) {
    return isdigit(c) || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F');
}

int toupper(int c) {
    if (c >= 'a' && c <= 'z') {
        return c - 'a' + 'A';
    }
    return c;
}

int tolower(int c) {
    if (c >= 'A' && c <= 'Z') {
        return c - 'A' + 'a';
    }
    return c;
}