# NebulaOS CMake Build System
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

cmake_minimum_required(VERSION 3.20)
project(NebulaOS VERSION 0.1.0 LANGUAGES C CXX ASM)

# =============================================================================
# Configuration
# =============================================================================
set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR i386)

# Default toolchain - can be overridden with -DCMAKE_TOOLCHAIN_FILE=
if(NOT CMAKE_C_COMPILER)
    set(CMAKE_C_COMPILER i686-elf-gcc)
    set(CMAKE_CXX_COMPILER i686-elf-g++)
    set(CMAKE_ASM_COMPILER nasm)
    set(CMAKE_OBJCOPY i686-elf-objcopy)
    set(CMAKE_STRIP i686-elf-strip)
    set(CMAKE_AR i686-elf-ar)
    set(CMAKE_RANLIB i686-elf-ranlib)
    set(CMAKE_NM i686-elf-nm)
    set(CMAKE_OBJDUMP i686-elf-objdump)
    set(CMAKE_READELF i686-elf-readelf)
endif()

# Build types
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

# =============================================================================
# Compiler Flags
# =============================================================================
# Kernel flags (freestanding, no standard library)
set(KERNEL_C_FLAGS
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

set(KERNEL_CXX_FLAGS "${KERNEL_C_FLAGS} -std=gnu++17")
set(KERNEL_ASM_FLAGS "-f elf32 -g")

# Userspace flags (minimal libc)
set(USER_C_FLAGS
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

set(USER_CXX_FLAGS "${USER_C_FLAGS} -std=gnu++17")
set(USER_ASM_FLAGS "-f elf32 -g")

# Linker flags
set(KERNEL_LD_FLAGS "-m elf_i386 -nostdlib -z max-page-size=0x1000")
set(USER_LD_FLAGS "-m elf_i386 -nostdlib")

# =============================================================================
# Paths
# =============================================================================
set(KERNEL_DIR ${CMAKE_SOURCE_DIR}/kernel)
set(DRIVERS_DIR ${CMAKE_SOURCE_DIR}/drivers)
set(SHELL_DIR ${CMAKE_SOURCE_DIR}/shell)
set(USERSPACE_DIR ${CMAKE_SOURCE_DIR}/userspace)
set(BOOT_DIR ${CMAKE_SOURCE_DIR}/boot)
set(BUILD_DIR ${CMAKE_BINARY_DIR})
set(ISO_DIR ${BUILD_DIR}/isodir)

# =============================================================================
# Find required tools
# =============================================================================
find_program(GRUB_MKRESCUE grub-mkrescue REQUIRED)
find_program(XORRISO xorriso REQUIRED)
find_program(QEMU qemu-system-i386 REQUIRED)
find_program(NASM nasm REQUIRED)

# =============================================================================
# Include directories
# =============================================================================
include_directories(
    ${CMAKE_SOURCE_DIR}
    ${KERNEL_DIR}
    ${DRIVERS_DIR}
    ${SHELL_DIR}
)

# =============================================================================
# Subdirectories
# =============================================================================
add_subdirectory(kernel)
add_subdirectory(drivers)
add_subdirectory(shell)
add_subdirectory(userspace)

# =============================================================================
# Kernel ELF target
# =============================================================================
add_custom_target(kernel.elf
    DEPENDS ${KERNEL_ELF}
    COMMENT "Building kernel.elf"
)

# =============================================================================
# Initrd creation
# =============================================================================
add_custom_target(initrd
    DEPENDS ${INITRD_FILE}
    COMMENT "Building initrd.cpio"
)

# =============================================================================
# ISO creation
# =============================================================================
add_custom_command(
    OUTPUT ${CMAKE_BINARY_DIR}/nebulaos.iso
    COMMAND mkdir -p ${ISO_DIR}/boot/grub
    COMMAND cp ${KERNEL_ELF} ${ISO_DIR}/boot/kernel.elf
    COMMAND cp ${BOOT_DIR}/grub/grub.cfg ${ISO_DIR}/boot/grub/grub.cfg
    COMMAND cp ${INITRD_FILE} ${ISO_DIR}/boot/initrd.cpio
    COMMAND ${GRUB_MKRESCUE} -o ${CMAKE_BINARY_DIR}/nebulaos.iso ${ISO_DIR}
    DEPENDS kernel.elf initrd ${BOOT_DIR}/grub/grub.cfg
    COMMENT "Creating nebulaos.iso"
)

add_custom_target(nebulaos.iso
    DEPENDS ${CMAKE_BINARY_DIR}/nebulaos.iso
    COMMENT "ISO target"
)

# =============================================================================
# Default target
# =============================================================================
add_custom_target(all
    DEPENDS nebulaos.iso
)

# =============================================================================
# Run targets
# =============================================================================
add_custom_target(run
    COMMAND ${QEMU} -m 64M -cdrom ${CMAKE_BINARY_DIR}/nebulaos.iso
    DEPENDS nebulaos.iso
    COMMENT "Running in QEMU"
)

add_custom_target(run-serial
    COMMAND ${QEMU} -m 64M -cdrom ${CMAKE_BINARY_DIR}/nebulaos.iso -display none -serial stdio
    DEPENDS nebulaos.iso
    COMMENT "Running in QEMU (serial only)"
)

add_custom_target(run-debug
    COMMAND ${QEMU} -m 64M -cdrom ${CMAKE_BINARY_DIR}/nebulaos.iso -display none -serial stdio -no-reboot -d int,cpu_reset -D ${BUILD_DIR}/qemu-debug.log
    DEPENDS nebulaos.iso
    COMMENT "Running in QEMU (debug)"
)

# =============================================================================
# Clean
# =============================================================================
add_custom_target(clean-all
    COMMAND rm -rf ${BUILD_DIR}
    COMMENT "Cleaning build directory"
)

# =============================================================================
# Install (for future package management)
# =============================================================================
install(FILES ${CMAKE_BINARY_DIR}/nebulaos.iso
    DESTINATION .
    RENAME nebulaos.iso
)