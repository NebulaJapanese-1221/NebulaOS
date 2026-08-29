use crate::common::idt;

#[no_mangle]
pub unsafe extern "C" fn init_idt64() {
    idt::init_idt64();
}
