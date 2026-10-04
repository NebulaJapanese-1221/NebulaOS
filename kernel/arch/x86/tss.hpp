// Task state segment for the NebulaOS x86 operating system.
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

#pragma once

namespace kernel::tss {

// Loads the task state segment into the GDT and into the task register.
//
// Without a TSS the CPU has no kernel stack to switch to and no interrupt
// stack table to fall back on, so an exception raised while the current stack is
// already unusable becomes a double fault, and a double fault raised for the
// same reason resets the machine. Giving the faulting vectors a dedicated stack
// is what keeps a stack overflow one recoverable screen instead of a triple
// fault.
void initialize();
void update_descriptor();
void reset_for_reinit();

// User-mode segment selectors (index 4 = 0x20, index 5 = 0x28, both with DPL=3).
constexpr unsigned short user_code_selector = 0x23;
constexpr unsigned short user_data_selector = 0x2B;

}