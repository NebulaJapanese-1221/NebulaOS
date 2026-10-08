# Shell CMakeLists.txt
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

set(SHELL_SOURCES
    shell.cpp
    window_manager.cpp
    apps/console.cpp
)

set(SHELL_HEADERS
    shell.hpp
    window_manager.hpp
    apps/console.hpp
)

foreach(cpp_src ${SHELL_SOURCES})
    get_filename_component(obj_name ${cpp_src} NAME_WE)
    # Replace path separators for object name
    string(REPLACE "/" "_" obj_name ${obj_name})
    add_custom_command(
        OUTPUT ${CMAKE_BINARY_DIR}/shell/${obj_name}.o
        COMMAND ${CMAKE_CXX_COMPILER} ${KERNEL_CXX_FLAGS} -I${CMAKE_SOURCE_DIR} -I${KERNEL_DIR} ${SHELL_DIR}/${cpp_src} -c -o ${CMAKE_BINARY_DIR}/shell/${obj_name}.o
        DEPENDS ${SHELL_DIR}/${cpp_src} ${SHELL_HEADERS} ${DRIVER_HEADERS} ${KERNEL_HEADERS}
        COMMENT "Compiling shell ${cpp_src}"
    )
    list(APPEND SHELL_OBJS ${CMAKE_BINARY_DIR}/shell/${obj_name}.o)
endforeach()

set(SHELL_OBJS ${SHELL_OBJS} PARENT_SCOPE)

add_custom_target(shell
    DEPENDS ${SHELL_OBJS}
    COMMENT "Building shell"
)