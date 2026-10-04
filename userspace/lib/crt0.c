// Userspace C runtime for NebulaOS.
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// See LICENCE for the full license text.

#include "syscalls.h"

extern int main(int argc, char** argv);
extern void _init(void);
extern void _fini(void);

void _start(int argc, char** argv) {
    if (main(argc, argv) == 0) {
        _exit(0);
    }
    _exit(1);
}

// Dummy init/fini
void _init(void) {}
void _fini(void) {}