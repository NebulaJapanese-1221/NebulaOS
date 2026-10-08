# Build rules for the NebulaOS x86 operating system.
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

# The canonical build system is CMake (see CMakeLists.txt). This
# Makefile is a thin wrapper so that `make` keeps working without
# duplicating the source list in two places.

CMAKE ?= cmake
BUILD_DIR ?= build

.PHONY: all clean run run-serial run-debug initrd kernel.elf nebulaos.iso

all: nebulaos.iso

$(BUILD_DIR)/Makefile: CMakeLists.txt
	$(CMAKE) -S . -B $(BUILD_DIR) -DCMAKE_TOOLCHAIN_FILE=toolchain/i686-elf.cmake

kernel.elf: $(BUILD_DIR)/Makefile
	$(CMAKE) --build $(BUILD_DIR) --target kernel.elf

initrd: $(BUILD_DIR)/Makefile
	$(CMAKE) --build $(BUILD_DIR) --target initrd

nebulaos.iso: kernel.elf initrd
	$(CMAKE) --build $(BUILD_DIR) --target nebulaos.iso

run: nebulaos.iso
	$(CMAKE) --build $(BUILD_DIR) --target run

run-serial: nebulaos.iso
	$(CMAKE) --build $(BUILD_DIR) --target run-serial

run-debug: nebulaos.iso
	$(CMAKE) --build $(BUILD_DIR) --target run-debug

clean:
	$(CMAKE) --build $(BUILD_DIR) --target clean-all 2>/dev/null || true
	rm -rf $(BUILD_DIR)