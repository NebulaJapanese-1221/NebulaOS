// PS/2 keyboard input for the NebulaOS x86 operating system.
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

#include "keyboard.hpp"

namespace {
// The state helpers below all run outside the drivers::keyboard namespace, so
// the modifier bits they operate on are pulled into scope here.
using drivers::keyboard::modifier_alt;
using drivers::keyboard::modifier_caps_lock;
using drivers::keyboard::modifier_control;
using drivers::keyboard::modifier_left_shift;
using drivers::keyboard::modifier_right_shift;

const unsigned char status_port = 0x64;
const unsigned char data_port = 0x60;

const unsigned char status_output_full = 0x01;
const unsigned char status_input_full = 0x20;

const unsigned char extended_prefix = 0xE0;
const unsigned char release_bit = 0x80;

const unsigned char code_left_shift = 0x2A;
const unsigned char code_right_shift = 0x36;
const unsigned char code_control = 0x1D;
const unsigned char code_alt = 0x38;
const unsigned char code_caps_lock = 0x3A;
const unsigned char code_space = 0x39;

// Scancode set 1 make codes run from the digit row through the slash key, and
// both tables are indexed by that code directly, so the row layout below has to
// match the specification exactly. Codes that hold no character, such as the
// bare modifier codes inside the range, are left zero. Getting these rows off
// by one silently mistypes the whole bottom letter row, so each row is spelled
// out rather than packed together.
const char unmodified[] =
    "\0\0"          // 0x00 unused, 0x01 escape
    "1234567890-="  // 0x02 .. 0x0D
    "\b\t"          // 0x0E backspace, 0x0F tab
    "qwertyuiop[]"  // 0x10 .. 0x1B
    "\n\0"          // 0x1C enter, 0x1D left control
    "asdfghjkl;'"   // 0x1E .. 0x28
    "`"             // 0x29 grave
    "\0"            // 0x2A left shift
    "\\"            // 0x2B backslash
    "zxcvbnm,./";   // 0x2C .. 0x35

const char shifted[] =
    "\0\0"
    "!@#$%^&*()_+"
    "\b\t"
    "QWERTYUIOP{}"
    "\n\0"
    "ASDFGHJKL:\""
    "~"
    "\0"
    "|"
    "ZXCVBNM<>?";

const unsigned char lowest_mapped_code = 0x02;
const unsigned char highest_mapped_code = 0x35;

unsigned char held_modifiers = 0;
bool extended_pending = false;

unsigned char read_port(unsigned short port) {
    unsigned char value;
    asm volatile("inb %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

bool is_letter(unsigned char code) {
    return (code >= 0x10 && code <= 0x19) || (code >= 0x1E && code <= 0x26) ||
           (code >= 0x2C && code <= 0x32);
}

bool shift_is_effective(unsigned char code, unsigned char modifiers) {
    if ((modifiers & (modifier_left_shift | modifier_right_shift)) != 0) {
        return true;
    }
    // Caps lock only inverts letters, so punctuation stays as typed.
    return (modifiers & modifier_caps_lock) != 0 && is_letter(code);
}

// Applies the edge to the tracked modifier state. Control and alt share a make
// code between their left and right instances, so the extended prefix is what
// separates them and the state is kept as a single bit.
void track_modifier(unsigned char code, bool released) {
    const bool clear = released;
    switch (code) {
    case code_left_shift:
        held_modifiers = static_cast<unsigned char>(
            clear ? (held_modifiers & ~modifier_left_shift)
                  : (held_modifiers | modifier_left_shift));
        break;
    case code_right_shift:
        held_modifiers = static_cast<unsigned char>(
            clear ? (held_modifiers & ~modifier_right_shift)
                  : (held_modifiers | modifier_right_shift));
        break;
    case code_control:
        held_modifiers = static_cast<unsigned char>(
            clear ? (held_modifiers & ~modifier_control)
                  : (held_modifiers | modifier_control));
        break;
    case code_alt:
        held_modifiers = static_cast<unsigned char>(
            clear ? (held_modifiers & ~modifier_alt)
                  : (held_modifiers | modifier_alt));
        break;
    case code_caps_lock:
        // Caps lock is a toggle rather than a held key, so only the press edge
        // changes it and there is no matching release.
        if (!released) {
            held_modifiers = static_cast<unsigned char>(
                held_modifiers ^ modifier_caps_lock);
        }
        break;
    default:
        break;
    }
}

bool is_modifier_code(unsigned char code) {
    return code == code_left_shift || code == code_right_shift ||
           code == code_control || code == code_alt || code == code_caps_lock;
}

// Control turns a letter into the classic control code for its position in
// the alphabet. The scancode rows are not laid out alphabetically, so the
// letter is looked up first rather than derived from the code.
char control_for(unsigned char code) {
    if (code < lowest_mapped_code || code > highest_mapped_code) {
        return '\0';
    }
    const char letter = unmodified[code];
    if (letter >= 'a' && letter <= 'z') {
        return static_cast<char>(letter - 'a' + 1);
    }
    if (letter >= 'A' && letter <= 'Z') {
        return static_cast<char>(letter - 'A' + 1);
    }
    return '\0';
}

char character_for(unsigned char code, unsigned char modifiers, bool extended) {
    if (extended) {
        return '\0';
    }
    if ((modifiers & modifier_control) != 0) {
        return control_for(code);
    }
    if (code == code_space) {
        return ' ';
    }
    if (code < lowest_mapped_code || code > highest_mapped_code) {
        return '\0';
    }
    return shift_is_effective(code, modifiers) ? shifted[code] : unmodified[code];
}
}

namespace drivers::keyboard {

void initialize() {
    // Discard anything the firmware left queued so a stale make code is never
    // mistaken for a real keystroke, and drop any half read extended prefix.
    while ((read_port(status_port) & status_output_full) != 0) {
        read_port(data_port);
    }
    held_modifiers = 0;
    extended_pending = false;
}

bool try_read_event(Event& event) {
    for (;;) {
        const unsigned char status = read_port(status_port);
        if ((status & status_output_full) == 0) {
            return false;
        }
        if ((status & status_input_full) != 0) {
            // The controller is overrun or the byte belongs to the mouse. The
            // byte still has to be drained or the controller stops reporting.
            read_port(data_port);
            continue;
        }

        const unsigned char raw = read_port(data_port);
        if (raw == extended_prefix) {
            // The byte that follows is qualified, so keep waiting for it.
            extended_pending = true;
            continue;
        }

        const bool released = (raw & release_bit) != 0;
        const unsigned char code = static_cast<unsigned char>(raw & ~release_bit);
        const bool extended = extended_pending;
        extended_pending = false;

        if (is_modifier_code(code)) {
            track_modifier(code, released);
        }

        event.code = code;
        event.modifiers = held_modifiers;
        event.released = released;
        event.extended = extended;
        event.character =
            released ? '\0' : character_for(code, held_modifiers, extended);
        return true;
    }
}

bool try_read_character(char& character) {
    Event event;
    while (try_read_event(event)) {
        if (event.character != '\0') {
            character = event.character;
            return true;
        }
    }
    return false;
}

}
