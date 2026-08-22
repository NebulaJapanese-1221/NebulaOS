// NebulaOS - Syscall Dispatch
// =============================
//
// System call interface

#include "../../common/include/syscall.h"
#include "../../common/include/nebula.h"
#include "../../common/include/stdint.h"
#include "../../kernel/common/include/vga.h"
#include "../../drivers/include/serial.h"
#include "../../drivers/include/pic.h"
#include "../../kernel/common/include/process.h"
#include "../../common/include/scheduler.h"
#include "../../lib/include/string.h"

static bool syscall_initialized = false;

void syscall_init(void) {
    syscall_initialized = true;
}

uint64_t syscall_dispatch(uint64_t num, uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5) {
    (void)arg2; (void)arg3; (void)arg4; (void)arg5;
    
    switch (num) {
        case SYS_WRITE: {
            const char* str = (const char*)arg1;
            uint32_t len = (uint32_t)arg2;
            for (uint32_t i = 0; i < len; i++) {
                serial_putchar(str[i]);
            }
            return len;
        }
        
        case SYS_READ: {
            (void)arg1; (void)arg2;
            return 0;
        }
        
        case SYS_OPEN: {
            (void)arg1; (void)arg2;
            return -1;
        }
        
        case SYS_CLOSE: {
            (void)arg1;
            return 0;
        }
        
        case SYS_EXIT: {
            (void)arg1;
            kernel_panic("Process exited");
            return 0;
        }
        
        case SYS_GETPID: {
            process_t* proc = process_get_current();
            return proc ? proc->pid : 0;
        }
        
        case SYS_YIELD: {
            scheduler_tick();
            return 0;
        }
        
        default:
            return (uint64_t)-1;
    }
}