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

pub fn reboot() -> ! {
    crate::graphics::system_screen::show("REBOOTING", "NebulaOS is restarting.");
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
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
        core::arch::asm!("out dx, ax", in("dx") 0x604u16, in("ax") 0x2000u16);
        core::arch::asm!("out dx, ax", in("dx") 0xB004u16, in("ax") 0x2000u16);
        loop {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}

unsafe fn wait_for_keyboard_controller() {
    for _ in 0..100_000 {
        let status: u8;
        core::arch::asm!("in al, dx", in("dx") 0x64u16, out("al") status);
        if status & 0x02 == 0 {
            return;
        }
        core::hint::spin_loop();
    }
}
