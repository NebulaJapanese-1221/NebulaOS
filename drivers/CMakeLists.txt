# Drivers CMakeLists.txt
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

set(DRIVER_SOURCES
    graphics.cpp
    keyboard.cpp
    mouse.cpp
    serial.cpp
    vga.cpp
)

set(DRIVER_HEADERS
    graphics.hpp
    keyboard.hpp
    mouse.hpp
    serial.hpp
    vga.hpp
)

foreach(cpp_src ${DRIVER_SOURCES})
    get_filename_component(obj_name ${cpp_src} NAME_WE)
    add_custom_command(
        OUTPUT ${CMAKE_BINARY_DIR}/drivers/${obj_name}.o
        COMMAND ${CMAKE_CXX_COMPILER} ${KERNEL_CXX_FLAGS} -I${CMAKE_SOURCE_DIR} -I${KERNEL_DIR} ${CMAKE_SOURCE_DIR}/drivers/${cpp_src} -c -o ${CMAKE_BINARY_DIR}/drivers/${obj_name}.o
        DEPENDS ${CMAKE_SOURCE_DIR}/drivers/${cpp_src} ${DRIVER_HEADERS} ${KERNEL_HEADERS}
        COMMENT "Compiling driver ${cpp_src}"
    )
    list(APPEND DRIVER_OBJS ${CMAKE_BINARY_DIR}/drivers/${obj_name}.o)
endforeach()

set(DRIVER_OBJS ${DRIVER_OBJS} PARENT_SCOPE)

add_custom_target(drivers
    DEPENDS ${DRIVER_OBJS}
    COMMENT "Building drivers"
)