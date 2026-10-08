# Userspace CMakeLists.txt
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
# Userspace Library (libc)
# =============================================================================
set(LIBC_SOURCES
    lib/crt0.c
    lib/libc.c
)

set(LIBC_HEADERS
    lib/libc.h
    lib/syscalls.h
)

set(LIBC_ASM_SOURCES
    lib/crt0.asm
)

# Assembly objects for libc
foreach(asm_src ${LIBC_ASM_SOURCES})
    get_filename_component(obj_name ${asm_src} NAME_WE)
    add_custom_command(
        OUTPUT ${CMAKE_BINARY_DIR}/userspace/${obj_name}.o
        COMMAND ${CMAKE_ASM_COMPILER} ${USER_ASM_FLAGS} -I${USERSPACE_DIR} ${USERSPACE_DIR}/${asm_src} -o ${CMAKE_BINARY_DIR}/userspace/${obj_name}.o
        DEPENDS ${USERSPACE_DIR}/${asm_src}
        COMMENT "Assembling userspace ${asm_src}"
    )
    list(APPEND LIBC_OBJS ${CMAKE_BINARY_DIR}/userspace/${obj_name}.o)
endforeach()

# C objects for libc
foreach(c_src ${LIBC_SOURCES})
    get_filename_component(obj_name ${c_src} NAME_WE)
    add_custom_command(
        OUTPUT ${CMAKE_BINARY_DIR}/userspace/${obj_name}.o
        COMMAND ${CMAKE_C_COMPILER} ${USER_C_FLAGS} -I${USERSPACE_DIR} ${USERSPACE_DIR}/${c_src} -c -o ${CMAKE_BINARY_DIR}/userspace/${obj_name}.o
        DEPENDS ${USERSPACE_DIR}/${c_src} ${LIBC_HEADERS}
        COMMENT "Compiling userspace ${c_src}"
    )
    list(APPEND LIBC_OBJS ${CMAKE_BINARY_DIR}/userspace/${obj_name}.o)
endforeach()

# =============================================================================
# Userspace Applications
# =============================================================================
set(APP_SOURCES
    bin/console.c
    bin/settings.c
)

foreach(c_src ${APP_SOURCES})
    get_filename_component(app_name ${c_src} NAME_WE)
    get_filename_component(obj_name ${c_src} NAME_WE)
    
    # Compile app object
    add_custom_command(
        OUTPUT ${CMAKE_BINARY_DIR}/userspace/${obj_name}.o
        COMMAND ${CMAKE_C_COMPILER} ${USER_C_FLAGS} -I${USERSPACE_DIR} ${USERSPACE_DIR}/${c_src} -c -o ${CMAKE_BINARY_DIR}/userspace/${obj_name}.o
        DEPENDS ${USERSPACE_DIR}/${c_src} ${LIBC_HEADERS}
        COMMENT "Compiling app ${c_src}"
    )
    
    # Link app executable
    add_custom_command(
        OUTPUT ${CMAKE_BINARY_DIR}/userspace/${app_name}
        COMMAND ${CMAKE_C_COMPILER} ${USER_LD_FLAGS} -T ${USERSPACE_DIR}/linker.ld
                ${CMAKE_BINARY_DIR}/userspace/crt0.o
                ${CMAKE_BINARY_DIR}/userspace/libc.o
                ${CMAKE_BINARY_DIR}/userspace/${obj_name}.o
                -o ${CMAKE_BINARY_DIR}/userspace/${app_name}
        DEPENDS ${CMAKE_BINARY_DIR}/userspace/${obj_name}.o
                ${CMAKE_BINARY_DIR}/userspace/crt0.o
                ${CMAKE_BINARY_DIR}/userspace/libc.o
                ${USERSPACE_DIR}/linker.ld
        COMMENT "Linking app ${app_name}"
    )
    
    list(APPEND APP_BINARIES ${CMAKE_BINARY_DIR}/userspace/${app_name})
endforeach()

# =============================================================================
# Initrd Creation
# =============================================================================
set(INITRD_FILE ${CMAKE_BINARY_DIR}/initrd.cpio)
add_custom_command(
    OUTPUT ${INITRD_FILE}
    COMMAND mkdir -p ${CMAKE_BINARY_DIR}/initrd_root
    COMMAND cp ${APP_BINARIES} ${CMAKE_BINARY_DIR}/initrd_root/
    COMMAND cd ${CMAKE_BINARY_DIR}/initrd_root && find . -print0 | cpio --null -o --format=newc > ${INITRD_FILE}
    DEPENDS ${APP_BINARIES}
    COMMENT "Creating initrd.cpio"
)

# =============================================================================
# Targets
# =============================================================================
add_custom_target(userspace-lib
    DEPENDS ${LIBC_OBJS}
    COMMENT "Building userspace libc"
)

add_custom_target(userspace-apps
    DEPENDS ${APP_BINARIES}
    COMMENT "Building userspace apps"
)

add_custom_target(userspace
    DEPENDS userspace-lib userspace-apps
    COMMENT "Building all userspace"
)

add_custom_target(initrd
    DEPENDS ${INITRD_FILE}
    COMMENT "Building initrd"
)

# Export for parent scope
set(INITRD_FILE ${INITRD_FILE} PARENT_SCOPE)
set(APP_BINARIES ${APP_BINARIES} PARENT_SCOPE)