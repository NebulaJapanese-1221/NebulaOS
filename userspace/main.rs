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
// MERCHANTABILITY OR FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with NebulaOS. If not, see <https://www.gnu.org/licenses/>.

//! Userspace binary - this is built as a separate ELF and embedded into the kernel.
//! The kernel loads this ELF and jumps to the entry point for each userspace app.

#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(asm_experimental_arch)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(lang_items)]
#![feature(panic_info_message)]
#![feature(linkage)]

extern crate alloc;

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    crate::libc::exit(1);
    loop {}
}

#[global_allocator]
static ALLOCATOR: crate::memory::allocator::GlobalAllocator = crate::memory::allocator::GlobalAllocator;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Userspace entry point - the kernel loads this ELF and jumps to the actual app entry point
    // This is just a placeholder; the real entry is determined by the ELF header
    loop {}
}

// Force inclusion of all userspace modules
mod apps {
    pub use crate::userspace::apps::*;
}
mod desktop {
    pub use crate::userspace::desktop::*;
}
mod storage {
    pub use crate::userspace::storage::*;
}