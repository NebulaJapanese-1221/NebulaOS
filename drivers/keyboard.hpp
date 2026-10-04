// PS/2 keyboard input interface for the NebulaOS x86 operating system.
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

namespace drivers::keyboard {

// Scancode set 1 key codes. Only keys that do not produce a plain character are
// named, because printable keys report their character on the event instead.
const unsigned char key_escape = 0x01;
const unsigned char key_backspace = 0x0E;
const unsigned char key_tab = 0x0F;
const unsigned char key_enter = 0x1C;
const unsigned char key_delete = 0x53;
const unsigned char key_home = 0x47;
const unsigned char key_end = 0x4F;
const unsigned char key_page_up = 0x49;
const unsigned char key_page_down = 0x51;
const unsigned char key_up = 0x48;
const unsigned char key_left = 0x4B;
const unsigned char key_right = 0x4D;
const unsigned char key_insert = 0x52;

// Modifier state carried alongside every key event. The shift bits are
// reported separately so a caller can tell which physical key is held, which
// matters for keys that behave differently per side.
enum Modifier : unsigned char {
    modifier_none = 0,
    modifier_left_shift = 1 << 0,
    modifier_right_shift = 1 << 1,
    modifier_control = 1 << 2,
    modifier_alt = 1 << 3,
    modifier_caps_lock = 1 << 4
};

struct Event {
    // Scancode set 1 make code with the release bit already stripped.
    unsigned char code;
    unsigned char modifiers;
    // True when the key was let go. Modifier keys report both edges so a
    // caller can track held state without keeping its own table.
    bool released;
    // True when the key arrived behind an extended prefix, which is how the
    // navigation cluster and the right hand modifiers are distinguished from
    // their numeric keypad namesakes.
    bool extended;
    // The character the key produced, or zero when it produces none. Never set
    // on a release, on a navigation key, or on a control combination.
    char character;
};

void initialize();

// Reads the next key event. Returns false when no complete event is queued,
// which leaves any pending state untouched for the following call.
bool try_read_event(Event& event);

// Convenience wrapper over try_read_event for callers that only want typed
// text. Releases, bare navigation keys and modified combinations are skipped.
bool try_read_character(char& character);

}
