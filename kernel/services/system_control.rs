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
