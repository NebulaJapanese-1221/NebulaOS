// PS/2 mouse input for the NebulaOS x86 operating system.
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

#include "mouse.hpp"

namespace {
unsigned int screen_width = 0;
unsigned int screen_height = 0;
unsigned int cursor_x = 0;
unsigned int cursor_y = 0;
unsigned char packet[3];
unsigned int packet_index = 0;
bool previous_left_pressed = false;

unsigned char read_port(unsigned short port) {
    unsigned char value;
    asm volatile("inb %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

void write_port(unsigned short port, unsigned char value) {
    asm volatile("outb %0, %1" : : "a"(value), "Nd"(port));
}

bool wait_input_clear() {
    for (unsigned int attempt = 0; attempt < 100000; ++attempt) {
        if ((read_port(0x64) & 0x02) == 0) {
            return true;
        }
    }
    return false;
}

bool wait_output_full() {
    for (unsigned int attempt = 0; attempt < 100000; ++attempt) {
        if ((read_port(0x64) & 0x01) != 0) {
            return true;
        }
    }
    return false;
}

bool send_controller_command(unsigned char command) {
    if (!wait_input_clear()) {
        return false;
    }
    write_port(0x64, command);
    return true;
}

bool send_mouse_byte(unsigned char value) {
    if (!send_controller_command(0xD4) || !wait_input_clear()) {
        return false;
    }
    write_port(0x60, value);
    return true;
}

bool read_mouse_response(unsigned char& response) {
    for (unsigned int attempt = 0; attempt < 100000; ++attempt) {
        const unsigned char status = read_port(0x64);
        if ((status & 0x01) != 0 && (status & 0x20) != 0) {
            response = read_port(0x60);
            return true;
        }
    }
    return false;
}

bool send_mouse_command(unsigned char command) {
    unsigned char response;
    return send_mouse_byte(command) && read_mouse_response(response) && response == 0xFA;
}

bool set_position(unsigned int& position, int delta, unsigned int limit) {
    const int updated = static_cast<int>(position) + delta;
    const unsigned int clamped = updated < 0 ? 0U :
        (static_cast<unsigned int>(updated) >= limit ? limit - 1 :
         static_cast<unsigned int>(updated));
    const bool changed = clamped != position;
    position = clamped;
    return changed;
}
}

namespace drivers::mouse {

bool initialize(unsigned int width, unsigned int height) {
    if (width == 0 || height == 0) {
        return false;
    }

    screen_width = width;
    screen_height = height;
    cursor_x = width / 2;
    cursor_y = height / 2;

    if (!send_controller_command(0xA8) ||
        !send_controller_command(0x20) || !wait_output_full()) {
        return false;
    }

    unsigned char configuration = read_port(0x60);
    configuration = static_cast<unsigned char>(configuration & ~0x22);
    if (!send_controller_command(0x60) || !wait_input_clear()) {
        return false;
    }
    write_port(0x60, configuration);

    if (!send_mouse_command(0xF6) || !send_mouse_command(0xF4)) {
        return false;
    }

    packet_index = 0;
    previous_left_pressed = false;
    return true;
}

bool poll(State& state) {
    bool changed = false;
    bool clicked = false;

    while ((read_port(0x64) & 0x21) == 0x21) {
        const unsigned char value = read_port(0x60);
        if (packet_index == 0 && (value & 0x08) == 0) {
            continue;
        }
        packet[packet_index++] = value;
        if (packet_index != 3) {
            continue;
        }
        packet_index = 0;

        const bool left_pressed = (packet[0] & 0x01) != 0;
        clicked = left_pressed && !previous_left_pressed;
        previous_left_pressed = left_pressed;

        if ((packet[0] & 0xC0) == 0) {
            const int delta_x = static_cast<signed char>(packet[1]);
            const int delta_y = -static_cast<signed char>(packet[2]);
            changed = set_position(cursor_x, delta_x, screen_width) || changed;
            changed = set_position(cursor_y, delta_y, screen_height) || changed;
        }
        changed = left_pressed != state.left_pressed || clicked || changed;
        state.left_pressed = left_pressed;
    }

    state.x = cursor_x;
    state.y = cursor_y;
    state.left_clicked = clicked;
    return changed;
}

}
