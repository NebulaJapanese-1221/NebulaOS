set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR i386)

set(TARGET_TRIPLET i686-elf)

set(CMAKE_C_COMPILER   clang   CACHE PATH "C compiler")
set(CMAKE_CXX_COMPILER clang++ CACHE PATH "C++ compiler")
set(CMAKE_ASM_COMPILER nasm                   CACHE PATH "Assembler")
set(CMAKE_AR           ar    CACHE PATH "Archiver")
set(CMAKE_RANLIB       ranlib CACHE PATH "Ranlib")
set(CMAKE_NM           nm    CACHE PATH "NM")
set(CMAKE_OBJDUMP      objdump CACHE PATH "Objdump")
set(CMAKE_OBJCOPY      objcopy CACHE PATH "Objcopy")
set(CMAKE_STRIP        strip  CACHE PATH "Strip")
set(CMAKE_READELF      readelf CACHE PATH "Readelf")

set(CMAKE_C_FLAGS_INIT "-target i686-pc-linux-gnu -m32 -ffreestanding -fno-exceptions -fno-stack-protector -fno-pic -fno-pie -nostdinc -Wall -Wextra -Werror -O2 -pipe -fdiagnostics-color=always")
set(CMAKE_CXX_FLAGS_INIT "-target i686-pc-linux-gnu -m32 -ffreestanding -fno-exceptions -fno-rtti -fno-stack-protector -fno-pic -fno-pie -nostdinc -nostdinc++ -Wall -Wextra -Werror -O2 -pipe -fdiagnostics-color=always -std=gnu++17")
set(CMAKE_ASM_FLAGS_INIT "-f elf32 -g")

set(CMAKE_EXE_LINKER_FLAGS_INIT "-target i686-pc-linux-gnu -m32 -m elf_i386 -nostdlib -z max-page-size=0x1000")
set(CMAKE_SHARED_LINKER_FLAGS_INIT "-target i686-pc-linux-gnu -m32 -m elf_i386 -nostdlib")
set(CMAKE_MODULE_LINKER_FLAGS_INIT "-target i686-pc-linux-gnu -m32 -m elf_i386 -nostdlib")

set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)

set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)

function(target_link_kernel target)
    target_link_options(${target} PRIVATE
        -target i686-pc-linux-gnu -m32
        -T ${CMAKE_SOURCE_DIR}/kernel/arch/x86/linker.ld
        -m elf_i386
        -nostdlib
        -z max-page-size=0x1000
    )
endfunction()

function(target_link_userspace target)
    target_link_options(${target} PRIVATE
        -target i686-pc-linux-gnu -m32
        -T ${CMAKE_SOURCE_DIR}/userspace/linker.ld
        -m elf_i386
        -nostdlib
    )
endfunction()
