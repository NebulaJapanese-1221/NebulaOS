// PC speaker interface for the NebulaOS x86 operating system.
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
// See LICENCE for the full license text.

#pragma once

#include <cstdint>

namespace drivers::speaker {

// Initialize the speaker hardware.
void initialize();

// Play a tone at the given frequency for the given duration, then silence.
void beep(unsigned int frequency_hz, unsigned int duration_ms);

// Silence the speaker immediately.
void silence();

} // namespace drivers::speaker
