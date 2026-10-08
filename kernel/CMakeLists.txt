# Kernel CMakeLists.txt
# Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
# See LICENCE for the full license text.

# =============================================================================
# Kernel Sources
# =============================================================================
set(KERNEL_SOURCES
    main.cpp
    cpio.cpp
    paging.cpp
    timer.cpp
    syscall.cpp
    heap.cpp
    pmm.cpp
    framebuffer.cpp
)

set(KERNEL_HEADERS
    cpio.hpp
    paging.hpp
    timer.hpp
    syscall.hpp
    heap.hpp
    pmm.hpp
    framebuffer.hpp
    multiboot.hpp
)

# Architecture-specific sources
set(KERNEL_ARCH_SOURCES
    arch/x86/tss.cpp
    arch/x86/interrupts.cpp
)

set(KERNEL_ARCH_HEADERS
    arch/x86/tss.hpp
    arch/x86/interrupts.hpp
    arch/x86/linker.ld
)

# Assembly sources
set(KERNEL_ASM_SOURCES
    arch/x86/entry.asm
    arch/x86/interrupts.asm
)

# Window manager sources
set(WM_SOURCES
    wm/wm.cpp
)

set(WM_HEADERS
    wm/wm.hpp
)

# =============================================================================
# Kernel Objects
# =============================================================================
# Assembly objects
foreach(asm_src ${KERNEL_ASM_SOURCES})
    get_filename_component(obj_name ${asm_src} NAME_WE)
    add_custom_command(
        OUTPUT ${CMAKE_BINARY_DIR}/kernel/${obj_name}.o
        COMMAND ${CMAKE_ASM_COMPILER} ${KERNEL_ASM_FLAGS} -I${KERNEL_DIR} ${KERNEL_DIR}/${asm_src} -o ${CMAKE_BINARY_DIR}/kernel/${obj_name}.o
        DEPENDS ${KERNEL_DIR}/${asm_src}
        COMMENT "Assembling ${asm_src}"
    )
    list(APPEND KERNEL_OBJS ${CMAKE_BINARY_DIR}/kernel/${obj_name}.o)
endforeach()

# C++ objects
foreach(cpp_src ${KERNEL_SOURCES} ${KERNEL_ARCH_SOURCES} ${WM_SOURCES})
    get_filename_component(obj_name ${cpp_src} NAME_WE)
    # Determine include directories based on path
    if(cpp_src MATCHES "^arch/")
        set(inc_dirs ${KERNEL_DIR})
    elseif(cpp_src MATCHES "^wm/")
        set(inc_dirs ${KERNEL_DIR})
    else()
        set(inc_dirs ${KERNEL_DIR})
    endif()
    add_custom_command(
        OUTPUT ${CMAKE_BINARY_DIR}/kernel/${obj_name}.o
        COMMAND ${CMAKE_CXX_COMPILER} ${KERNEL_CXX_FLAGS} -I${KERNEL_DIR} -I${CMAKE_SOURCE_DIR} ${KERNEL_DIR}/${cpp_src} -c -o ${CMAKE_BINARY_DIR}/kernel/${obj_name}.o
        DEPENDS ${KERNEL_DIR}/${cpp_src} ${KERNEL_HEADERS} ${KERNEL_ARCH_HEADERS} ${WM_HEADERS}
        COMMENT "Compiling ${cpp_src}"
    )
    list(APPEND KERNEL_OBJS ${CMAKE_BINARY_DIR}/kernel/${obj_name}.o)
endforeach()

# =============================================================================
# Kernel ELF
# =============================================================================
set(KERNEL_ELF ${CMAKE_BINARY_DIR}/kernel.elf)
add_custom_command(
    OUTPUT ${KERNEL_ELF}
    COMMAND ${CMAKE_CXX_COMPILER} ${KERNEL_LD_FLAGS} -T ${KERNEL_DIR}/arch/x86/linker.ld ${KERNEL_OBJS} -o ${KERNEL_ELF}
    DEPENDS ${KERNEL_OBJS} ${KERNEL_DIR}/arch/x86/linker.ld
    COMMENT "Linking kernel.elf"
)

# Make kernel.elf available to parent scope
set(KERNEL_ELF ${KERNEL_ELF} PARENT_SCOPE)
set(KERNEL_OBJS ${KERNEL_OBJS} PARENT_SCOPE)

# =============================================================================
# Custom targets for convenience
# =============================================================================
add_custom_target(kernel-objects
    DEPENDS ${KERNEL_OBJS}
    COMMENT "Building kernel objects"
)

add_custom_target(kernel
    DEPENDS ${KERNEL_ELF}
    COMMENT "Building kernel"
)