// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

// Programmable Interrupt Controller remapping

const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const PIC_EOI: u8 = 0x20;

const ICW1_INIT: u8 = 0x11;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

fn port_byte_in(port: u16) -> u8 {
    let val: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") val, in("dx") port);
    }
    val
}

pub fn remap(offset1: u8, offset2: u8) {
    let a1 = port_byte_in(PIC1_DATA);
    let a2 = port_byte_in(PIC2_DATA);

    port_byte_out(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
    port_byte_out(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);

    port_byte_out(PIC1_DATA, offset1);
    port_byte_out(PIC2_DATA, offset2);

    port_byte_out(PIC1_DATA, 0x04);
    port_byte_out(PIC2_DATA, 0x02);

    port_byte_out(PIC1_DATA, ICW4_8086);
    port_byte_out(PIC2_DATA, ICW4_8086);

    port_byte_out(PIC1_DATA, a1);
    port_byte_out(PIC2_DATA, a2);
}

pub fn eoi(irq: u8) {
    if irq >= 8 {
        port_byte_out(PIC2_COMMAND, PIC_EOI);
    }
    port_byte_out(PIC1_COMMAND, PIC_EOI);
}

pub fn mask_irq(irq: u8) {
    let port = if irq < 8 { PIC1_DATA } else { PIC2_DATA };
    let mask = 1 << (irq % 8);
    let val = port_byte_in(port) | mask;
    port_byte_out(port, val);
}

pub fn unmask_irq(irq: u8) {
    if irq >= 8 {
        unmask_irq(2);
    }
    let port = if irq < 8 { PIC1_DATA } else { PIC2_DATA };
    let mask = 1 << (irq % 8);
    let val = port_byte_in(port) & !mask;
    port_byte_out(port, val);
}
