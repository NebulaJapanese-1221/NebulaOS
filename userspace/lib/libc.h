// NebulaOS POSIX-Compatible C Library
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

#ifndef _NEBULA_LIBC_H
#define _NEBULA_LIBC_H

#include <cstdint>
#include <cstddef>
#include <cstdarg>

#ifdef __cplusplus
extern "C" {
#endif

// =============================================================================
// Types
// =============================================================================
typedef int errno_t;
typedef long ssize_t;
typedef unsigned int mode_t;
typedef unsigned int uid_t;
typedef unsigned int gid_t;
typedef long off_t;
typedef unsigned int pid_t;
typedef int fd_t;

// =============================================================================
// Error codes (POSIX)
// =============================================================================
#define EPERM        1   // Operation not permitted
#define ENOENT       2   // No such file or directory
#define ESRCH        3   // No such process
#define EINTR        4   // Interrupted system call
#define EIO          5   // I/O error
#define ENXIO        6   // No such device or address
#define E2BIG        7   // Argument list too long
#define ENOEXEC      8   // Exec format error
#define EBADF        9   // Bad file number
#define ECHILD      10   // No child processes
#define EAGAIN      11   // Try again
#define ENOMEM      12   // Out of memory
#define EACCES      13   // Permission denied
#define EFAULT      14   // Bad address
#define ENOTBLK     15   // Block device required
#define EBUSY       16   // Device or resource busy
#define EEXIST      17   // File exists
#define EXDEV       18   // Cross-device link
#define ENODEV      19   // No such device
#define ENOTDIR     20   // Not a directory
#define EISDIR      21   // Is a directory
#define EINVAL      22   // Invalid argument
#define ENFILE      23   // File table overflow
#define EMFILE      24   // Too many open files
#define ENOTTY      25   // Not a typewriter
#define ETXTBSY     26   // Text file busy
#define EFBIG       27   // File too large
#define ENOSPC      28   // No space left on device
#define ESPIPE      29   // Illegal seek
#define EROFS       30   // Read-only file system
#define EMLINK      31   // Too many links
#define EPIPE       32   // Broken pipe
#define EDOM        33   // Math argument out of domain
#define ERANGE      34   // Math result not representable
#define ENOMSG      42   // No message of desired type
#define EIDRM       43   // Identifier removed
#define EDEADLK     35   // Resource deadlock would occur
#define ENOLCK      37   // No record locks available
#define ENOSYS      38   // Function not implemented
#define ENOTEMPTY   39   // Directory not empty
#define ELOOP       40   // Too many symbolic links
#define EWOULDBLOCK EAGAIN
#define ENOMSG      42
#define EILSEQ      84   // Illegal byte sequence

// =============================================================================
// String functions
// =============================================================================
size_t strlen(const char* s);
char* strcpy(char* dest, const char* src);
char* strncpy(char* dest, const char* src, size_t n);
char* strcat(char* dest, const char* src);
char* strncat(char* dest, const char* src, size_t n);
int strcmp(const char* a, const char* b);
int strncmp(const char* a, const char* b, size_t n);
int strcoll(const char* a, const char* b);
char* strchr(const char* s, int c);
char* strrchr(const char* s, int c);
size_t strspn(const char* s, const char* accept);
size_t strcspn(const char* s, const char* reject);
char* strpbrk(const char* s, const char* accept);
char* strstr(const char* haystack, const char* needle);
char* strtok(char* s, const char* delim);
char* strdup(const char* s);
char* strndup(const char* s, size_t n);
size_t strxfrm(char* dest, const char* src, size_t n);
void* memset(void* s, int c, size_t n);
void* memcpy(void* dest, const void* src, size_t n);
void* memmove(void* dest, const void* src, size_t n);
int memcmp(const void* a, const void* b, size_t n);
void* memchr(const void* s, int c, size_t n);
void* memrchr(const void* s, int c, size_t n);

// =============================================================================
// stdio
// =============================================================================
typedef struct {
    int fd;
    char* buffer;
    size_t buffer_size;
    size_t buffer_pos;
    int flags;
} FILE;

#define EOF (-1)
#define BUFSIZ 512
#define FILENAME_MAX 256
#define FOPEN_MAX 16
#define TMP_MAX 256
#define L_tmpnam 256
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

#define _IOFBF 0x01  // Full buffering
#define _IOLBF 0x02  // Line buffering
#define _IONBF 0x04  // No buffering
#define _IOEOF 0x08  // End of file
#define _IOERR 0x10  // Error

int puts(const char* s);
int fputs(const char* s, FILE* stream);
int printf(const char* fmt, ...);
int fprintf(FILE* stream, const char* fmt, ...);
int sprintf(char* out, const char* fmt, ...);
int snprintf(char* out, size_t size, const char* fmt, ...);
int vprintf(const char* fmt, va_list ap);
int vfprintf(FILE* stream, const char* fmt, va_list ap);
int vsprintf(char* out, const char* fmt, va_list ap);
int vsnprintf(char* out, size_t size, const char* fmt, va_list ap);
int putchar(int c);
int fputc(int c, FILE* stream);
int puts(const char* s);
int fgets(char* buf, int n, FILE* stream);
int getchar(void);
int ungetc(int c, FILE* stream);
int fflush(FILE* stream);
int fclose(FILE* stream);
FILE* fopen(const char* path, const char* mode);
FILE* freopen(const char* path, const char* mode, FILE* stream);
void perror(const char* s);
void clearerr(FILE* stream);
int feof(FILE* stream);
int ferror(FILE* stream);
int fgetc(FILE* stream);
int fgetpos(FILE* stream, long* pos);
int fseek(FILE* stream, long offset, int whence);
int fsetpos(FILE* stream, const long* pos);
long ftell(FILE* stream);
void rewind(FILE* stream);
int remove(const char* path);
int rename(const char* old, const char* new_path);
FILE* tmpfile(void);
char* tmpnam(char* s);
size_t fread(void* ptr, size_t size, size_t n, FILE* stream);
size_t fwrite(const void* ptr, size_t size, size_t n, FILE* stream);
int fscanf(FILE* stream, const char* fmt, ...);
int scanf(const char* fmt, ...);
int sscanf(const char* s, const char* fmt, ...);

// =============================================================================
// stdlib
// =============================================================================
int atoi(const char* s);
long atol(const char* s);
long long atoll(const char* s);
double atof(const char* s);
void* malloc(size_t size);
void free(void* ptr);
void* calloc(size_t count, size_t size);
void* realloc(void* ptr, size_t size);
void abort(void);
void exit(int status);
void _exit(int status);
int atexit(void (*func)(void));
int abs(int n);
long labs(long n);
long long llabs(long long n);
int rand(void);
void srand(unsigned int seed);
int mblen(const char* s, size_t n);
int mbtowc(int* pwc, const char* s, size_t n);
int wctomb(char* s, int wchar);
int system(const char* command);
char* getenv(const char* name);
int setenv(const char* name, const char* value, int overwrite);
int unsetenv(const char* name);
int posix_memalign(void** memptr, size_t alignment, size_t size);
void* aligned_alloc(size_t alignment, size_t size);

// =============================================================================
// unistd
// =============================================================================
int open(const char* path, int flags, ...);
int close(int fd);
ssize_t read(int fd, void* buf, size_t count);
ssize_t write(int fd, const void* buf, size_t count);
off_t lseek(int fd, off_t offset, int whence);
int fstat(int fd, void* stat);
int stat(const char* path, void* stat);
int lstat(const char* path, void* stat);
int access(const char* path, int mode);
int dup(int fd);
int dup2(int oldfd, int newfd);
int pipe(int fds[2]);
int isatty(int fd);
int truncate(const char* path, off_t length);
int ftruncate(int fd, off_t length);
ssize_t readlink(const char* path, char* buf, size_t bufsiz);
int symlink(const char* target, const char* linkpath);
int link(const char* oldpath, const char* newpath);
int unlink(const char* path);
int rmdir(const char* path);
int mkdir(const char* path, mode_t mode);
int chdir(const char* path);
int fchdir(int fd);
char* getcwd(char* buf, size_t size);
int chroot(const char* path);
int fsync(int fd);
int fdatasync(int fd);
int fcntl(int fd, int cmd, ...);
int ioctl(int fd, unsigned long request, ...);
int swapon(const char* path, int swap_flags);
int swapoff(const char* path);
int reboot(int magic, int magic2, unsigned int cmd, void* arg);
int gethostname(char* name, size_t len);
int sethostname(const char* name, size_t len);
int getpid(void);
int getppid(void);
int getuid(void);
int geteuid(void);
int getgid(void);
int getegid(void);
int setuid(uid_t uid);
int setgid(gid_t gid);
int setpgid(pid_t pid, pid_t pgid);
pid_t getpgid(pid_t pid);
pid_t setsid(void);
pid_t getsid(pid_t pid);
int usleep(useconds_t usec);
int sleep(unsigned int seconds);
int nanosleep(const struct timespec* req, struct timespec* rem);

// File open flags
#define O_RDONLY    0x0000
#define O_WRONLY    0x0001
#define O_RDWR      0x0002
#define O_CREAT     0x0040
#define O_EXCL      0x0080
#define O_TRUNC     0x0200
#define O_APPEND    0x0400
#define O_NONBLOCK  0x0800
#define O_DSYNC     0x1000
#define O_SYNC      0x2000
#define O_ASYNC     0x4000

// Access modes
#define R_OK 4
#define W_OK 2
#define X_OK 1
#define F_OK 0

// =============================================================================
// sys/stat
// =============================================================================
struct stat {
    unsigned int st_dev;
    unsigned int st_ino;
    mode_t st_mode;
    unsigned int st_nlink;
    uid_t st_uid;
    gid_t st_gid;
    unsigned int st_rdev;
    off_t st_size;
    unsigned long st_atime;
    unsigned long st_mtime;
    unsigned long st_ctime;
    unsigned int st_blksize;
    unsigned int st_blocks;
};

#define S_IFMT   0xF000
#define S_IFSOCK 0xC000
#define S_IFLNK  0xA000
#define S_IFREG  0x8000
#define S_IFBLK  0x6000
#define S_IFDIR  0x4000
#define S_IFCHR  0x2000
#define S_IFIFO  0x1000

#define S_ISLNK(m)  (((m) & S_IFMT) == S_IFLNK)
#define S_ISREG(m)  (((m) & S_IFMT) == S_IFREG)
#define S_ISDIR(m)  (((m) & S_IFMT) == S_IFDIR)
#define S_ISCHR(m)  (((m) & S_IFMT) == S_IFCHR)
#define S_ISBLK(m)  (((m) & S_IFMT) == S_IFBLK)
#define S_ISFIFO(m) (((m) & S_IFMT) == S_IFIFO)
#define S_ISSOCK(m) (((m) & S_IFMT) == S_IFSOCK)

#define S_IRWXU 0x01C0
#define S_IRUSR 0x0100
#define S_IWUSR 0x0080
#define S_IXUSR 0x0040
#define S_IRWXG 0x0038
#define S_IRGRP 0x0020
#define S_IWGRP 0x0010
#define S_IXGRP 0x0008
#define S_IRWXO 0x0007
#define S_IROTH 0x0004
#define S_IWOTH 0x0002
#define S_IXOTH 0x0001

// =============================================================================
// dirent.h
// =============================================================================
struct dirent {
    unsigned int d_ino;
    unsigned int d_off;
    unsigned short d_reclen;
    char d_name[256];
};

typedef struct {
    int fd;
    struct dirent entry;
} DIR;

DIR* opendir(const char* name);
struct dirent* readdir(DIR* dirp);
void rewinddir(DIR* dirp);
int closedir(DIR* dirp);
int dirfd(DIR* dirp);

// =============================================================================
// fcntl.h
// =============================================================================
#define F_DUPFD    0
#define F_GETFD    1
#define F_SETFD    2
#define F_GETFL    3
#define F_SETFL    4
#define F_GETLK    5
#define F_SETLK    6
#define F_SETLKW   7

#define FD_CLOEXEC 1

// =============================================================================
// time.h
// =============================================================================
struct timespec {
    long tv_sec;
    long tv_nsec;
};

struct timeval {
    long tv_sec;
    long tv_usec;
};

struct tm {
    int tm_sec;
    int tm_min;
    int tm_hour;
    int tm_mday;
    int tm_mon;
    int tm_year;
    int tm_wday;
    int tm_yday;
    int tm_isdst;
};

time_t time(time_t* t);
int gettimeofday(struct timeval* tv, void* tz);
clock_t clock(void);
double difftime(time_t t1, time_t t0);
time_t mktime(struct tm* tm);
char* asctime(const struct tm* tm);
char* ctime(const time_t* timep);
struct tm* gmtime(const time_t* timep);
struct tm* localtime(const time_t* timep);
size_t strftime(char* s, size_t max, const char* fmt, const struct tm* tm);

// =============================================================================
// Signal handling
// =============================================================================
#define SIGINT     1
#define SIGILL     2
#define SIGABRT    3
#define SIGFPE     4
#define SIGKILL    5
#define SIGSEGV    6
#define SIGTERM    7
#define SIGUSR1    8
#define SIGUSR2    9
#define SIGCHLD   10
#define SIGCONT   11
#define SIGSTOP   12
#define SIGTSTP   13

typedef void (*sighandler_t)(int);
sighandler_t signal(int signum, sighandler_t handler);
int raise(int signum);
int kill(pid_t pid, int signum);

// =============================================================================
// setjmp.h
// =============================================================================
typedef struct {
    unsigned int ebx;
    unsigned int ecx;
    unsigned int edx;
    unsigned int esi;
    unsigned int edi;
    unsigned int ebp;
    unsigned int esp;
    unsigned int eip;
} jmp_buf[1];

int setjmp(jmp_buf env);
void longjmp(jmp_buf env, int val);

// =============================================================================
// assert.h
// =============================================================================
#ifdef NDEBUG
#define assert(x) ((void)0)
#else
#define assert(x) do { \
    if (!(x)) { \
        perror("Assertion failed: " #x); \
        abort(); \
    } \
} while (0)
#endif

// =============================================================================
// pthread.h (minimal)
// =============================================================================
typedef unsigned long pthread_t;
typedef struct {
    int locked;
} pthread_mutex_t;

#define PTHREAD_MUTEX_INITIALIZER { 0 }

int pthread_create(pthread_t* thread, void* attr,
                   void* (*start)(void*), void* arg);
int pthread_join(pthread_t thread, void** retval);
int pthread_detach(pthread_t thread);
int pthread_exit(void* retval);
int pthread_mutex_init(pthread_mutex_t* mutex, void* attr);
int pthread_mutex_destroy(pthread_mutex_t* mutex);
int pthread_mutex_lock(pthread_mutex_t* mutex);
int pthread_mutex_unlock(pthread_mutex_t* mutex);

// =============================================================================
// framebuffer device
// =============================================================================
typedef struct {
    unsigned int buffer;
    unsigned int width;
    unsigned int height;
    unsigned int pitch;
    unsigned char bpp;
    unsigned char type;
    unsigned char red_position;
    unsigned char red_mask_size;
    unsigned char green_position;
    unsigned char green_mask_size;
    unsigned char blue_position;
    unsigned char blue_mask_size;
} framebuffer_info_t;

int open_framebuffer(framebuffer_info_t* info);
void close_framebuffer(int fd);
unsigned int fb_pixel(unsigned int x, unsigned int y);
void fb_put_pixel(unsigned int x, unsigned int y, unsigned int color);
void fb_fill_rect(unsigned int x, unsigned int y, unsigned int w, unsigned int h, unsigned int color);
void fb_present(void);
unsigned int fb_width(void);
unsigned int fb_height(void);

// =============================================================================
// Misc
// =============================================================================
void* memset_bytes(void* s, int c, size_t n);
int isspace(int c);
int isdigit(int c);
int isalpha(int c);
int isalnum(int c);
int isupper(int c);
int islower(int c);
int ispunct(int c);
int isgraph(int c);
int isprint(int c);
int iscntrl(int c);
int isxdigit(int c);
int toupper(int c);
int tolower(int c);

#ifdef __cplusplus
}
#endif

#endif // _NEBULA_LIBC_H