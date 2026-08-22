// NebulaOS - C++ RTTI stubs
// ===========================
//
// Minimal RTTI support

namespace std {
    class type_info {
    public:
        virtual ~type_info() {}
        virtual bool operator==(const type_info&) const { return false; }
        virtual bool operator!=(const type_info& other) const { return !(*this == other); }
        virtual bool before(const type_info&) const { return false; }
        virtual const char* name() const { return ""; }
    };
}

extern "C" int __cxa_demangle(const char*, char*, size_t*, int*) {
    return 0;
}