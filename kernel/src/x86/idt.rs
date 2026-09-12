use crate::common::idt;

extern "C" {
    static mut isr_table: [unsafe extern "C" fn(); 32];
    static mut irq_table: [unsafe extern "C" fn(); 16];
}

#[no_mangle]
pub unsafe extern "C" fn init_idt() {
    idt::init_idt64();
}
