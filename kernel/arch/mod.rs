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

// Architecture-specific modules for x86 (i686).

#[cfg(target_arch = "x86")]
pub mod gdt;
#[cfg(target_arch = "x86")]
pub mod idt;
#[cfg(target_arch = "x86")]
pub mod lapic;
#[cfg(target_arch = "x86")]
pub mod pic;
#[cfg(target_arch = "x86")]
pub mod smp;
#[cfg(target_arch = "x86")]
pub mod tss;

#[cfg(target_arch = "x86")]
#[allow(unused_imports)]
pub use gdt::*;
#[cfg(target_arch = "x86")]
#[allow(unused_imports)]
pub use idt::*;
#[cfg(target_arch = "x86")]
#[allow(unused_imports)]
pub use lapic::*;
#[cfg(target_arch = "x86")]
#[allow(unused_imports)]
pub use pic::*;
#[cfg(target_arch = "x86")]
#[allow(unused_imports)]
pub use smp::*;
#[cfg(target_arch = "x86")]
#[allow(unused_imports)]
pub use tss::*;
