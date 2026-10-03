// 16550A UART output for the NebulaOS x86 operating system.
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

#include "serial.hpp"

namespace {
const unsigned short register_receive = 0;
const unsigned short register_interrupt = 1;
const unsigned short register_fifo = 2;
const unsigned short register_line = 3;
const unsigned short register_modem = 4;
const unsigned short register_status = 5;

const unsigned char status_transmitter_empty = 0x20;
const unsigned char control_dlab = 0x80;
const unsigned char control_8n1 = 0x03;
const unsigned char fifo_enabled = 0x07;
const unsigned char fifo_clear_receiver = 0x02;
const unsigned char fifo_clear_transmitter = 0x04;
const unsigned char modem_ready = 0x03;
const unsigned char modem_interrupt_gate = 0x08;

const unsigned int input_clock_hz = 115200;
const unsigned int baud_rate = 115200;
const unsigned int maximum_spins = 100000;

unsigned short selected_port = 0;
bool port_ready = false;

unsigned char read_register(unsigned short offset) {
    const unsigned short port = static_cast<unsigned short>(selected_port + offset);
    unsigned char value = 0;
    asm volatile("inb %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

void write_register(unsigned short offset, unsigned char value) {
    const unsigned short port = static_cast<unsigned short>(selected_port + offset);
    asm volatile("outb %0, %1" : : "a"(value), "Nd"(port));
}

bool wait_for_transmitter() {
    for (unsigned int spin = 0; spin < maximum_spins; ++spin) {
        if ((read_register(register_status) & status_transmitter_empty) != 0) {
            return true;
        }
    }
    return false;
}
}

namespace drivers::serial {

bool initialize(unsigned short port) {
    selected_port = port;
    port_ready = false;

    write_register(register_interrupt, 0x00);
    write_register(register_line, control_dlab);
    const unsigned int divisor = input_clock_hz / baud_rate;
    write_register(register_receive, static_cast<unsigned char>(divisor & 0xFF));
    write_register(register_interrupt, static_cast<unsigned char>(divisor >> 8));
    write_register(register_line, control_8n1);
    write_register(register_fifo, fifo_enabled | fifo_clear_receiver | fifo_clear_transmitter);
    write_register(register_modem, modem_ready | modem_interrupt_gate);

    port_ready = wait_for_transmitter();
    if (port_ready) {
        write_line("");
        write_line("NEBULAOS SERIAL CONSOLE ONLINE");
    }
    return port_ready;
}

bool is_ready() {
    return port_ready;
}

void write(char character) {
    if (!port_ready) {
        return;
    }
    if (character == '\n') {
        if (!wait_for_transmitter()) {
            port_ready = false;
            return;
        }
        write_register(register_receive, '\r');
    }
    if (!wait_for_transmitter()) {
        port_ready = false;
        return;
    }
    write_register(register_receive, static_cast<unsigned char>(character));
}

void write(const char* text) {
    for (unsigned int index = 0; text != nullptr && text[index] != '\0'; ++index) {
        write(text[index]);
    }
}

void write_newline() {
    write('\n');
}

void write_line(const char* text) {
    write(text);
    write('\n');
}

void write_hex(unsigned int value) {
    const char digits[] = "0123456789ABCDEF";
    char buffer[11];
    buffer[0] = '0';
    buffer[1] = 'x';
    for (unsigned int index = 0; index < 8; ++index) {
        buffer[2 + index] = digits[(value >> ((7 - index) * 4)) & 0xF];
    }
    buffer[10] = '\0';
    write(buffer);
}

void write_decimal(unsigned int value) {
    char buffer[11];
    unsigned int index = 0;
    do {
        buffer[index] = static_cast<char>('0' + (value % 10));
        ++index;
        value /= 10;
    } while (value != 0);
    for (unsigned int position = index; position > 0; --position) {
        write(buffer[position - 1]);
    }
}

}