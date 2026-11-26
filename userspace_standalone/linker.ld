/* SPDX-License-Identifier: GPL-3.0-only */
/* Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com> */
/*
 * This file is part of NebulaOS.
 *
 * NebulaOS is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License.
 *
 * NebulaOS is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.
 */

OUTPUT_FORMAT(elf32-i386)
ENTRY(_start)

SECTIONS
{
    . = 0x8000;
    .ap_trampoline : {
        KEEP(*(.ap_trampoline))
    }

    . = 0x10000;
    .text : {
        *(.text.boot)
        KEEP(*(.multiboot2))
        *(.text)
        *(.text.*)
    }
    . = ALIGN(4K);
    .rodata : {
        *(.rodata .rodata.*)
    }
    . = ALIGN(4K);
    .data : {
        *(.data .data.*)
    }
    . = ALIGN(4K);
    .bss : {
        __bss_start = .;
        *(COMMON)
        *(.bss .bss.*)
        . = ALIGN(4K);
        __bss_end = .;
    }
    /DISCARD/ : {
        *(.eh_frame)
        *(.note.GNU-stack)
        *(.gcc_except_table)
    }
}
