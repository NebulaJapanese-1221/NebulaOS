#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

// The BIOS loads this at 0x7c00. 
// We use inline assembly to set up segments and switch to 32-bit Protected Mode.
global_asm!(
    ".code16",
    ".global _start",
    "_start:",
    "cli",                          // Disable interrupts
    "xor ax, ax",                   // Zero out segment registers
    "mov ds, ax",
    "mov es, ax",
    "mov ss, ax",
    "mov sp, 0x7c00",               // Set stack pointer below bootloader

    // Print "Loading..." in text mode (initial boot process)
    "mov si, offset loading_msg",
    "print_loop:",
    "lodsb",
    "or al, al",
    "jz switch_vga",
    "mov ah, 0x0e",
    "int 0x10",
    "jmp print_loop",

    "switch_vga:",
    // Set VBE mode 0x4118 (1024x768x32 with Linear Framebuffer bit)
    "mov ax, 0x4f02",
    "mov bx, 0x4118",
    "int 0x10",

    "loading_msg: .asciz \"NebulaOS is loading...\"",

    // Transition to 32-bit Protected Mode
    "lgdt [gdt_descriptor]",        // Load Global Descriptor Table
    "mov eax, cr0",
    "or eax, 0x1",                  // Set PE (Protection Enable) bit
    "mov cr0, eax",

    "push 0x08",                    // Push code segment selector (CS)
    "push offset start32",          // Push instruction pointer (IP)
    "retf",                         // Far return: pops IP then CS, switching to 32-bit mode

    ".code32",
    "start32:",
    "mov ax, 0x10",                 // Set up data segments for 32-bit
    "mov ds, ax",
    "mov es, ax",
    "mov ss, ax",
    
    // In a real OS, we would load the kernel from disk here.
    // For now, we jump to a placeholder address where the kernel will live.
    "mov eax, 0x10000",             // Fix: Use a register for absolute jump
    "jmp eax",

    ".align 4",
    "gdt_start:",
    "    .quad 0x0000000000000000", // Null descriptor
    "    .quad 0x00cf9a000000ffff", // Code segment
    "    .quad 0x00cf92000000ffff", // Data segment
    "gdt_end:",
    "gdt_descriptor:",
    "    .word gdt_end - gdt_start - 1",
    "    .long gdt_start",

    ".org 510",                     // Fill until 510 bytes
    ".word 0xaa55"                  // Boot signature
);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
