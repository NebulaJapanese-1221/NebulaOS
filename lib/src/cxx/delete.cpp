// NebulaOS - C++ delete operators
// ==================================
//
// Sized delete operators

void operator delete(void* ptr, size_t) noexcept {
    free(ptr);
}

void operator delete[](void* ptr, size_t) noexcept {
    free(ptr);
}