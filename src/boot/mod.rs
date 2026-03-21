use core::arch::global_asm;

global_asm!(
    r#"
    .section .multiboot_header
    .align 4
    .long 0x1BADB002        # Multiboot Magic
    .long 0x07              # Flags: Align modules, Mem info, Video info
    .long -(0x1BADB002 + 0x07) # Checksum

    # These are GRUB-specific fields for setting a graphics mode.
    .long 0 # header_addr
    .long 0 # load_addr
    .long 0 # load_end_addr
    .long 0 # bss_end_addr
    .long 0 # entry_addr

    .long 0 # mode_type = 0 for linear graphics buffer
    .long 800  # width
    .long 600  # height
    .long 32   # depth

    .section .text
    .global _start
    _start:
        /* 1. Setup the stack pointer */
        lea esp, [stack_top]

        /* 2. Call the kernel main function (defined in src/kernel/mod.rs) */
        /* Push the multiboot info pointer (in EBX) as the first argument */
        push ebx
        call kernel_main

        /* 3. If kernel returns, loop indefinitely */
        cli
    1:  hlt
        jmp 1b

    .section .bss
    .align 16
    stack_bottom:
        /* Reserve 64KB for stack */
        .skip 65536
    stack_top:
    "#
);
