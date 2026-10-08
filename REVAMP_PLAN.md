# NebulaOS Revamp Plan

## Vision
Iteratively refactor NebulaOS into a modern, well-structured 32-bit x86 hobby OS suitable for learning and experimentation with:
- Clean kernel architecture with proper separation of concerns
- Modern graphics/compositor stack
- POSIX-compatible userspace
- Extensible driver framework

## Phase 1: Foundation & Build System (Weeks 1-2)
- [ ] Migrate from Make to CMake with cross-compilation support
- [ ] Add proper target definitions (kernel, drivers, libc, apps)
- [ ] Set up CI/CD (GitHub Actions) for build verification
- [ ] Add clang-format, clang-tidy, static analysis
- [ ] Document build process and toolchain requirements

## Phase 2: Core Kernel Refactoring (Weeks 3-6)
### Memory Management
- [ ] Redesign PMM with buddy allocator for better fragmentation handling
- [ ] Implement proper virtual memory manager (VMM) with address spaces
- [ ] Add kernel slab allocator for common object sizes
- [ ] Implement userspace page fault handling (COW, demand paging)
- [ ] Add memory mapping syscalls (mmap, munmap, mprotect)

### Process & Thread Management
- [ ] Design process/thread structures with proper ownership
- [ ] Implement scheduler (start with round-robin, then priority-based)
- [ ] Add context switching with FPU/SSE save/restore
- [ ] Implement thread synchronization (mutex, condition variables, semaphores)
- [ ] Add process creation (fork, exec, wait) and termination

### IPC
- [ ] Design message passing interface (ports/channels)
- [ ] Implement shared memory regions
- [ ] Add notification/event mechanism
- [ ] Create syscall interface for IPC

## Phase 3: Graphics & Window System (Weeks 7-10)
### Graphics Stack
- [ ] Separate framebuffer driver from drawing API
- [ ] Implement double/triple buffering with vsync
- [ ] Add hardware cursor support
- [ ] Create compositor architecture (damage tracking, dirty rects)
- [ ] Add basic 2D acceleration (blitting, scaling, alpha blending)

### Window Manager
- [ ] Redesign as userspace compositor + kernel WM protocol
- [ ] Implement proper window hierarchy (parent/child, z-order)
- [ ] Add window decorations as separate surfaces
- [ ] Implement input focus model with focus follows mouse/click
- [ ] Add drag/resize with visual feedback
- [ ] Support multiple monitors (future)

## Phase 4: Userspace & Libc (Weeks 11-14)
### Libc Expansion
- [ ] Complete POSIX.1-2008 coverage (file I/O, dirent, stat, etc.)
- [ ] Add proper stdio buffering (FILE*, fopen, fclose, fprintf)
- [ ] Implement malloc/free with dlmalloc or similar
- [ ] Add pthreads support (create, join, mutex, cond, TLS)
- [ ] Add dynamic loader (ld.so) for shared libraries

### Userspace Programs
- [ ] Create init process and service manager
- [ ] Build shell with job control, pipes, redirection
- [ ] Port basic utilities (ls, cat, echo, mkdir, rm, cp, mv)
- [ ] Add terminal emulator with VT100/ANSI support
- [ ] Create GUI apps using new WM protocol

## Phase 5: Drivers & Hardware (Weeks 15-20)
### Core Subsystems
- [ ] PCI enumeration and configuration space access
- [ ] Interrupt routing (APIC, MSI/MSI-X)
- [ ] DMA framework (bounce buffers, scatter-gather)

### Storage
- [ ] AHCI/SATA driver with NCQ
- [ ] NVMe driver
- [ ] Filesystem: FAT32 (read/write), ext2 (read)

### Input/Display
- [ ] USB HID (keyboard, mouse, touchscreen)
- [ ] PS/2 improvements (hotplug, multimedia keys)
- [ ] Basic GPU drivers (VBE, simple framebuffer)

### Network
- [ ] RTL8139 / e1000 / virtio-net
- [ ] TCP/IP stack (lwIP or custom)
- [ ] Socket API in libc

## Phase 6: Polish & Testing (Ongoing)
- [ ] Unit tests for kernel components
- [ ] Integration tests for syscalls
- [ ] Fuzzing for parsers (ELF, filesystem, network)
- [ ] Documentation (kernel internals, driver model, syscalls)
- [ ] Performance profiling and optimization

## Architecture Principles
1. **Separation of concerns**: Kernel handles hardware, scheduling, IPC; userspace handles policy
2. **Capability-based security**: Objects referenced by capabilities, not raw pointers
3. **Async by default**: Non-blocking APIs, completion-based I/O
4. **Testability**: Pure functions where possible, dependency injection
5. **Documentation**: Every public API has header docs; internals have design docs

## Technical Debt to Address
- Global state in kernel (move to per-CPU/per-process structures)
- Hardcoded limits (MAX_WINDOWS, fixed arrays)
- No error handling framework (Result<T> types)
- Inconsistent naming conventions
- Missing bounds checking in userspace-facing APIs
- No kernel logging framework (ring buffer, severity levels)

## Milestones
| Milestone | Target | Criteria |
|-----------|--------|----------|
| M1: Build System | Week 2 | `cmake --build build` produces bootable ISO |
| M2: Memory & Scheduling | Week 6 | Multiple processes run concurrently |
| M3: Graphics Stack | Week 10 | Composited desktop with 60fps |
| M4: Userspace | Week 14 | Shell + basic utilities work |
| M5: Drivers | Week 20 | Boot on real hardware (or QEMU with PCI) |
| M6: Self-Hosting | Future | Build NebulaOS on NebulaOS |

## Risk Mitigation
- **Scope creep**: Strict phase gates; new ideas go to backlog
- **Rewrites stall**: Keep old code working until new replaces it
- **Debugging difficulty**: Add kernel debugger (GDB stub) early
- **Knowledge gaps**: Spike tasks for unfamiliar areas (PCI, AHCI, etc.)

## References
- OSDev Wiki, Intel SDM, AMD APM
- "Operating Systems: Three Easy Pieces" (OSTEP)
- Linux kernel source (for API design patterns)
- seL4, Fuchsia, Redox (modern microkernel ideas)