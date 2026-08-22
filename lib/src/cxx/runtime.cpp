// NebulaOS - C++ Runtime Stubs
// ==============================
//
// Minimal C++ runtime support for kernel

#include <stddef.h>

extern "C" void* malloc(size_t size);
extern "C" void free(void* ptr);

void* operator new(size_t size) {
    return malloc(size);
}

void* operator new[](size_t size) {
    return malloc(size);
}

void operator delete(void* ptr) {
    free(ptr);
}

void operator delete[](void* ptr) {
    free(ptr);
}

void* operator new(size_t, void* ptr) {
    return ptr;
}

void* operator new[](size_t, void* ptr) {
    return ptr;
}

void operator delete(void*, size_t) {
}

void operator delete[](void*, size_t) {
}

// -----------------------------------------------------------------------------
// C++ ABI stubs (minimal, no RTTI/exceptions)
// -----------------------------------------------------------------------------

void* __dso_handle = 0;

int __cxa_atexit(void (*func)(void*), void* arg, void* dso) {
    (void)func;
    (void)arg;
    (void)dso;
    return 0;
}

namespace __cxxabiv1 {
class __class_type_info {
public:
    virtual ~__class_type_info() {}
    virtual bool __is_pointer_p() const { return false; }
    virtual bool __is_function_p() const { return false; }
    virtual bool __has_base(const __class_type_info* base, void*& base_ptr) const { (void)base; (void)base_ptr; return false; }
};

class __si_class_type_info : public __class_type_info {
public:
    const __class_type_info* __base_type;
    bool __is_pointer_p() const override { return false; }
    bool __is_function_p() const override { return false; }
    bool __has_base(const __class_type_info* base, void*& base_ptr) const override {
        (void)base_ptr;
        if (__base_type == base) {
            return true;
        }
        return __base_type->__has_base(base, base_ptr);
    }
};
}

void* __cxa_pure_virtual() {
    return 0;
}

void __gxx_personality_v0() {
}

void _Unwind_Resume() {
}
