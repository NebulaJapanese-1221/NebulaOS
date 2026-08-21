// NebulaOS - Standard Argument Header
// =================================
//
// Variable argument list handling

#ifndef NEBULAOS_LIB_STDARG_H
#define NEBULAOS_LIB_STDARG_H

#ifdef __cplusplus
extern "C" {
#endif

// Variable argument list type
typedef char* va_list;

// Start variable argument list
#define va_start(ap, last) \
    ((void)(ap = (va_list)&(last) + sizeof(last)))

// Get next argument
#define va_arg(ap, type) \
    (*(type*)((ap += sizeof(type)) - sizeof(type)))

// End variable argument list
#define va_end(ap) \
    ((void)(ap = (va_list)0))

#ifdef __cplusplus
}
#endif

#endif // NEBULAOS_LIB_STDARG_H
