// NebulaOS - C++ exception stubs
// =================================
//
// Stubs for exception handling

extern "C" void __cxa_throw(void*, void*, void (*)(void*)) {
    for (;;);
}

extern "C" void* __cxa_allocate_exception(size_t) {
    return NULL;
}

extern "C" void __cxa_begin_catch(void*) {
    for (;;);
}

extern "C" void __cxa_end_catch(void) {
}

extern "C" void __cxa_pure_virtual(void) {
    for (;;);
}

extern "C" int __cxa_guard_acquire(long long* guard) {
    return !*(char*)guard;
}

extern "C" void __cxa_guard_release(long long* guard) {
    *(char*)guard = 1;
}