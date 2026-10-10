# NebulaOS Cross-Compilation Toolchain
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

# This toolchain file configures CMake for cross-compiling to i686-elf
# Usage: cmake -DCMAKE_TOOLCHAIN_FILE=toolchain/i686-elf.cmake ..

set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR i386)

# Target triplet
set(TARGET_TRIPLET i686-elf)

# Compiler locations - can be overridden via environment or cache
set(CMAKE_C_COMPILER   gcc   CACHE PATH "C compiler")
set(CMAKE_CXX_COMPILER g++   CACHE PATH "C++ compiler")
set(CMAKE_ASM_COMPILER nasm                   CACHE PATH "Assembler")
set(CMAKE_AR           ${TARGET_TRIPLET}-ar    CACHE PATH "Archiver")
set(CMAKE_RANLIB       ${TARGET_TRIPLET}-ranlib CACHE PATH "Ranlib")
set(CMAKE_NM           ${TARGET_TRIPLET}-nm    CACHE PATH "NM")
set(CMAKE_OBJDUMP      ${TARGET_TRIPLET}-objdump CACHE PATH "Objdump")
set(CMAKE_OBJCOPY      ${TARGET_TRIPLET}-objcopy CACHE PATH "Objcopy")
set(CMAKE_STRIP        ${TARGET_TRIPLET}-strip  CACHE PATH "Strip")
set(CMAKE_READELF      ${TARGET_TRIPLET}-readelf CACHE PATH "Readelf")

# Flags for the target
set(CMAKE_C_FLAGS_INIT
    "-m32"
    "-ffreestanding"
    "-fno-exceptions"
    "-fno-rtti"
    "-fno-stack-protector"
    "-fno-pic"
    "-fno-pie"
    "-nostdinc"
    "-nostdinc++"
    "-Wall"
    "-Wextra"
    "-Werror"
    "-O2"
    "-pipe"
    "-fdiagnostics-color=always"
)

set(CMAKE_CXX_FLAGS_INIT "${CMAKE_C_FLAGS_INIT} -std=gnu++17")
set(CMAKE_ASM_FLAGS_INIT "-f elf32 -g")

# Linker flags
set(CMAKE_EXE_LINKER_FLAGS_INIT "-m elf_i386 -nostdlib -z max-page-size=0x1000")
set(CMAKE_SHARED_LINKER_FLAGS_INIT "-m elf_i386 -nostdlib")
set(CMAKE_MODULE_LINKER_FLAGS_INIT "-m elf_i386 -nostdlib")

# Find root mode - never search host paths for libraries/headers
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)

# Sysroot (if using a sysroot directory)
# set(CMAKE_SYSROOT /path/to/i686-elf/sysroot)

# Prevent CMake from trying to run target executables during build
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)

# Custom commands for kernel-specific linking
function(target_link_kernel target)
    target_link_options(${target} PRIVATE
        -T ${CMAKE_SOURCE_DIR}/kernel/arch/x86/linker.ld
        -m elf_i386
        -nostdlib
        -z max-page-size=0x1000
    )
endfunction()

function(target_link_userspace target)
    target_link_options(${target} PRIVATE
        -T ${CMAKE_SOURCE_DIR}/userspace/linker.ld
        -m elf_i386
        -nostdlib
    )
endfunction()