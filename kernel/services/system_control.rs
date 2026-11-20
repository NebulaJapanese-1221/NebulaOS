// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
//
// This file is part of NebulaOS.
//
// NebulaOS is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License.
//
// NebulaOS is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

use crate::arch::smp;

pub fn reboot() -> ! {
    crate::graphics::system_screen::show("REBOOTING", "NebulaOS is restarting.");
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));

        // Try ACPI reboot via FADT reset register
        if let Some(reset_reg) = smp::acpi_reset_register() {
            let reset_value = smp::acpi_reset_value().unwrap_or(0x6E);
            port_byte_out(reset_reg as u16, reset_value);
            for _ in 0..1_000_000 {
                core::hint::spin_loop();
            }
        }

        // Fallback: legacy keyboard controller reboot
        core::arch::asm!("out dx, al", in("dx") 0xCF9u16, in("al") 0x02u8);
        for _ in 0..100_000 {
            core::hint::spin_loop();
        }
        core::arch::asm!("out dx, al", in("dx") 0xCF9u16, in("al") 0x06u8);
        for _ in 0..100_000 {
            core::hint::spin_loop();
        }
        wait_for_keyboard_controller();
        core::arch::asm!("out dx, al", in("dx") 0x64u16, in("al") 0xFEu8);
        let invalid_idt = [0u8; 6];
        core::arch::asm!(
            "lidt [{}]",
            "int3",
            in(reg) invalid_idt.as_ptr(),
            options(noreturn)
        );
    }
}

pub fn shutdown() -> ! {
    crate::graphics::system_screen::show("SHUTTING DOWN", "It is now safe to power off.");
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));

        // Try ACPI shutdown via PM1a_CNT_BLK
        if let Some(pm1a_cnt) = smp::acpi_pm1a_cnt_blk() {
            let slp_typ = smp::acpi_sleep_control().unwrap_or(5);
            // Write SLP_TYP << 10 | SLP_EN (bit 13) to PM1a_CNT
            let val = ((slp_typ as u16) << 10) | (1 << 13);
            port_word_out(pm1a_cnt, val);
            for _ in 0..1_000_000 {
                core::hint::spin_loop();
            }
        }

        // Fallback: legacy APM shutdown
        core::arch::asm!("out dx, ax", in("dx") 0x604u16, in("ax") 0x2000u16);
        core::arch::asm!("out dx, ax", in("dx") 0xB004u16, in("ax") 0x2000u16);
        loop {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}

unsafe fn wait_for_keyboard_controller() {
    for _ in 0..100_000 {
        let status = port_byte_in(0x64);
        if status & 0x02 == 0 {
            return;
        }
        core::hint::spin_loop();
    }
}

fn port_byte_in(port: u16) -> u8 {
    let val: u8;
    unsafe {
        core::arch::asm!("in al, dx", out("al") val, in("dx") port);
    }
    val
}

fn port_byte_out(port: u16, data: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") data);
    }
}

fn port_word_out(port: u16, data: u16) {
    unsafe {
        core::arch::asm!("out dx, ax", in("dx") port, in("ax") data);
    }
}
