/* Linker script for the NebulaOS x86 operating system.
 * Copyright (C) 2026 NebulaJapanese-1221 <nebulajapanese@gmail.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 * See LICENCE for the full license text.
 */

OUTPUT_FORMAT(elf32-i386)
OUTPUT_ARCH(i386)
ENTRY(_start)

/* GRUB still loads the image at its physical address, but everything after the
 * Multiboot header is linked to run from the higher half. Each section carries
 * an explicit load address below this offset so the physical layout stays
 * contiguous, and kernel_image_end therefore stays a physical address. */
KERNEL_VIRTUAL_BASE = 0xC0000000;

SECTIONS {
    . = 0x00100000;
    kernel_image_start = .;
    .multiboot ALIGN(4) : { KEEP(*(.multiboot)) }

    . += KERNEL_VIRTUAL_BASE;

    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_VIRTUAL_BASE) { *(.text*) }
    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_VIRTUAL_BASE) { *(.rodata*) }
    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_VIRTUAL_BASE) { *(.data*) }
    .bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_VIRTUAL_BASE) { *(.bss*) *(COMMON) }

    kernel_image_end = ADDR(.bss) + SIZEOF(.bss) - KERNEL_VIRTUAL_BASE;
}
