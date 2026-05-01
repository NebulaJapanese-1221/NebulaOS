//! NebulaOS JIT Compiler IR and Engine.
//! This module handles the translation of architecture-independent bytecode into native machine code.

use alloc::vec::Vec;
use core::fmt;

/// Represents a virtual register in the JIT environment.
/// These are mapped to physical registers during the native compilation phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VReg(pub u8);

/// Architecture-independent instructions for the NebulaOS JIT.
#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    /// Load an immediate value into a register: VReg = Const
    LoadImm(VReg, usize),
    /// Load from memory: VReg(Dst) = [VReg(Addr)]
    Load(VReg, VReg),
    /// Store to memory: [VReg(Addr)] = VReg(Src)
    Store(VReg, VReg),
    /// Move value between registers: VReg(Dst) = VReg(Src)
    Mov(VReg, VReg),
    /// Add two registers: VReg(Dst) = VReg(Src1) + VReg(Src2)
    Add(VReg, VReg, VReg),
    /// Sub two registers: VReg(Dst) = VReg(Src1) - VReg(Src2)
    Sub(VReg, VReg, VReg),
    /// Multiply two registers: VReg(Dst) = VReg(Src1) * VReg(Src2)
    Mul(VReg, VReg, VReg),
    /// Bitwise AND: VReg(Dst) = VReg(Src1) & VReg(Src2)
    And(VReg, VReg, VReg),
    /// Bitwise OR: VReg(Dst) = VReg(Src1) | VReg(Src2)
    Or(VReg, VReg, VReg),
    /// Bitwise XOR: VReg(Dst) = VReg(Src1) ^ VReg(Src2)
    Xor(VReg, VReg, VReg),
    /// Shift Left: VReg(Dst) = VReg(Src) << VReg(Count)
    Shl(VReg, VReg, VReg),
    /// Shift Right: VReg(Dst) = VReg(Src) >> VReg(Count)
    Shr(VReg, VReg, VReg),
    /// Compare two registers and store result in flags
    Cmp(VReg, VReg),
    /// Unconditional jump to a relative instruction offset
    Jmp(usize),
    /// Jump if equal to a relative instruction offset
    JmpEq(usize),
    /// Jump if not equal to a relative instruction offset
    JmpNe(usize),
    /// Jump if less than (signed) to a relative instruction offset
    JmpLt(usize),
    /// Jump if greater than (signed) to a relative instruction offset
    JmpGt(usize),
    /// Jump if greater or equal (signed) to a relative instruction offset
    JmpGe(usize),
    /// Jump if less or equal (signed) to a relative instruction offset
    JmpLe(usize),
    /// Call a kernel function by index (e.g., GUI draw calls)
    SysCall(u32),
    /// Return from the JIT function
    Ret,
}

pub struct JitFunction {
    pub ir_code: Vec<Instruction>,
    pub native_code: Vec<u8>,
}

impl JitFunction {
    pub fn new() -> Self {
        Self {
            ir_code: Vec::new(),
            native_code: Vec::new(),
        }
    }

    /// Basic translation stub. 
    /// This will eventually iterate through ir_code and emit x86/x86_64 opcodes.
    pub fn compile(&mut self) -> Result<(), &'static str> {
        self.native_code.clear();

        for instr in &self.ir_code {
            match instr {
                Instruction::Ret => {
                    // Native x86 'ret' opcode
                    self.native_code.push(0xC3);
                }
                Instruction::LoadImm(reg, val) => {
                    #[cfg(target_arch = "x86")]
                    {
                        // MOV reg, imm32
                        // Opcode 0xB8 + reg_index
                        self.native_code.push(0xB8 + reg.0);
                        let bytes = (*val as u32).to_le_bytes();
                        self.native_code.extend_from_slice(&bytes);
                    }
                    #[cfg(target_arch = "x86_64")]
                    {
                        // MOVABS rax, imm64 (Standard 64-bit load)
                        // REX.W + 0xB8 + reg_index
                        self.native_code.push(0x48);
                        self.native_code.push(0xB8 + reg.0);
                        let bytes = (*val as u64).to_le_bytes();
                        self.native_code.extend_from_slice(&bytes);
                    }
                }
                Instruction::Load(dst, addr) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        // MOV r64, [r64] -> REX.W + 0x8B + ModR/M
                        let mut rex = 0x48;
                        if dst.0 > 7 { rex |= 0x04; } // REX.R (dst)
                        if addr.0 > 7 { rex |= 0x01; } // REX.B (base addr)
                        self.native_code.push(rex);
                        self.native_code.push(0x8B);
                        // Mod=00 (register indirect), Reg=dst, R/M=addr
                        // Note: addr.0 == 4 (RSP) or 5 (RBP) requires special handling (SIB/Disp)
                        self.native_code.push(((dst.0 & 7) << 3) | (addr.0 & 7));
                    }
                }
                Instruction::Store(addr, src) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        // MOV [r64], r64 -> REX.W + 0x89 + ModR/M
                        let mut rex = 0x48;
                        if src.0 > 7 { rex |= 0x04; } // REX.R (src)
                        if addr.0 > 7 { rex |= 0x01; } // REX.B (base addr)
                        self.native_code.push(rex);
                        self.native_code.push(0x89);
                        // Mod=00 (register indirect), Reg=src, R/M=addr
                        self.native_code.push(((src.0 & 7) << 3) | (addr.0 & 7));
                    }
                }
                Instruction::Mov(dst, src) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        if dst.0 != src.0 {
                            // MOV r/m64, r64 -> REX.W + 0x89 + ModR/M
                            let mut rex = 0x48; // REX.W
                            if src.0 > 7 { rex |= 0x04; } // REX.R bit
                            if dst.0 > 7 { rex |= 0x01; } // REX.B bit
                            self.native_code.push(rex);
                            self.native_code.push(0x89);
                            self.native_code.push(0xC0 | ((src.0 & 7) << 3) | (dst.0 & 7));
                        }
                    }
                }
                Instruction::Add(dst, src1, src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        // 1. Ensure dst contains src1: MOV dst, src1
                        if dst.0 != src1.0 {
                            let mut rex = 0x48;
                            if src1.0 > 7 { rex |= 0x04; }
                            if dst.0 > 7 { rex |= 0x01; }
                            self.native_code.push(rex);
                            self.native_code.push(0x89);
                            self.native_code.push(0xC0 | ((src1.0 & 7) << 3) | (dst.0 & 7));
                        }
                        // 2. Perform addition: ADD dst, src2
                        // ADD r/m64, r64 -> REX.W + 0x01 + ModR/M
                        let mut rex = 0x48;
                        if src2.0 > 7 { rex |= 0x04; }
                        if dst.0 > 7 { rex |= 0x01; }
                        self.native_code.push(rex);
                        self.native_code.push(0x01);
                        self.native_code.push(0xC0 | ((src2.0 & 7) << 3) | (dst.0 & 7));
                    }
                }
                Instruction::Sub(dst, src1, src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        // 1. Ensure dst contains src1: MOV dst, src1
                        if dst.0 != src1.0 {
                            let mut rex = 0x48;
                            if src1.0 > 7 { rex |= 0x04; }
                            if dst.0 > 7 { rex |= 0x01; }
                            self.native_code.push(rex);
                            self.native_code.push(0x89);
                            self.native_code.push(0xC0 | ((src1.0 & 7) << 3) | (dst.0 & 7));
                        }
                        // 2. Perform subtraction: SUB dst, src2
                        // SUB r/m64, r64 -> REX.W + 0x29 + ModR/M
                        let mut rex = 0x48;
                        if src2.0 > 7 { rex |= 0x04; }
                        if dst.0 > 7 { rex |= 0x01; }
                        self.native_code.push(rex);
                        self.native_code.push(0x29);
                        self.native_code.push(0xC0 | ((src2.0 & 7) << 3) | (dst.0 & 7));
                    }
                }
                Instruction::Mul(dst, src1, src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        if dst.0 != src1.0 {
                            self.native_code.extend_from_slice(&[0x48, 0x89, 0xC0 | ((src1.0 & 7) << 3) | (dst.0 & 7)]);
                        }
                        // IMUL r64, r/m64 -> REX.W + 0x0F 0xAF + ModR/M
                        self.native_code.push(0x48);
                        self.native_code.push(0x0F);
                        self.native_code.push(0xAF);
                        self.native_code.push(0xC0 | ((src2.0 & 7) << 3) | (dst.0 & 7));
                    }
                }
                Instruction::And(dst, src1, src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        if dst.0 != src1.0 { self.native_code.extend_from_slice(&[0x48, 0x89, 0xC0 | ((src1.0 & 7) << 3) | (dst.0 & 7)]); }
                        self.native_code.extend_from_slice(&[0x48, 0x21, 0xC0 | ((src2.0 & 7) << 3) | (dst.0 & 7)]);
                    }
                }
                Instruction::Or(dst, src1, src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        if dst.0 != src1.0 { self.native_code.extend_from_slice(&[0x48, 0x89, 0xC0 | ((src1.0 & 7) << 3) | (dst.0 & 7)]); }
                        self.native_code.extend_from_slice(&[0x48, 0x09, 0xC0 | ((src2.0 & 7) << 3) | (dst.0 & 7)]);
                    }
                }
                Instruction::Xor(dst, src1, src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        if dst.0 != src1.0 { self.native_code.extend_from_slice(&[0x48, 0x89, 0xC0 | ((src1.0 & 7) << 3) | (dst.0 & 7)]); }
                        self.native_code.extend_from_slice(&[0x48, 0x31, 0xC0 | ((src2.0 & 7) << 3) | (dst.0 & 7)]);
                    }
                }
                Instruction::Cmp(v1, v2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        self.native_code.extend_from_slice(&[0x48, 0x39, 0xC0 | ((v2.0 & 7) << 3) | (v1.0 & 7)]);
                    }
                    #[cfg(target_arch = "x86")]
                    {
                        // CMP r/m32, r32 -> 0x39 + ModR/M
                        self.native_code.push(0x39);
                        self.native_code.push(0xC0 | ((v2.0 & 7) << 3) | (v1.0 & 7));
                    }
                }
                Instruction::Jmp(target_idx) => {
                    let offset = instr_offsets[*target_idx] as isize - next_instr_addr as isize;
                    self.native_code.push(0xE9);
                    self.native_code.extend_from_slice(&(offset as i32).to_le_bytes());
                }
                Instruction::JmpEq(target_idx) => {
                    let offset = instr_offsets[*target_idx] as isize - next_instr_addr as isize;
                    self.native_code.extend_from_slice(&[0x0F, 0x84]); // JE rel32
                    self.native_code.extend_from_slice(&(offset as i32).to_le_bytes());
                }
                Instruction::JmpNe(target_idx) => {
                    let offset = instr_offsets[*target_idx] as isize - next_instr_addr as isize;
                    self.native_code.extend_from_slice(&[0x0F, 0x85]); // JNE rel32
                    self.native_code.extend_from_slice(&(offset as i32).to_le_bytes());
                }
                Instruction::JmpLt(target_idx) => {
                    let offset = instr_offsets[*target_idx] as isize - next_instr_addr as isize;
                    self.native_code.extend_from_slice(&[0x0F, 0x8C]); // JL rel32
                    self.native_code.extend_from_slice(&(offset as i32).to_le_bytes());
                }
                Instruction::JmpGt(target_idx) => {
                    let offset = instr_offsets[*target_idx] as isize - next_instr_addr as isize;
                    self.native_code.extend_from_slice(&[0x0F, 0x8F]); // JG rel32
                    self.native_code.extend_from_slice(&(offset as i32).to_le_bytes());
                }
                Instruction::JmpGe(target_idx) => {
                    let offset = instr_offsets[*target_idx] as isize - next_instr_addr as isize;
                    self.native_code.extend_from_slice(&[0x0F, 0x8D]); // JGE rel32
                    self.native_code.extend_from_slice(&(offset as i32).to_le_bytes());
                }
                Instruction::JmpLe(target_idx) => {
                    let offset = instr_offsets[*target_idx] as isize - next_instr_addr as isize;
                    self.native_code.extend_from_slice(&[0x0F, 0x8E]); // JLE rel32
                    self.native_code.extend_from_slice(&(offset as i32).to_le_bytes());
                }
                Instruction::SysCall(id) => {
                    // Step 1: Load the Syscall ID into EAX/RAX (Register 0)
                    // Opcode 0xB8 + reg_idx is "MOV reg, imm32". 
                    // Since the ID is u32, this works on both x86 and x86_64.
                    self.native_code.push(0xB8);
                    self.native_code.extend_from_slice(&id.to_le_bytes());

                    // Step 2: Trigger the syscall interrupt (INT 0x80)
                    // NebulaOS uses int 0x80 as the gateway to the kernel syscall dispatcher.
                    self.native_code.push(0xCD);
                    self.native_code.push(0x80);
                }
                _ => {
                    // TODO: Implement other IR to Native mappings
                    return Err("Instruction not yet supported by JIT backend");
                }
            }
        }

        if self.native_code.is_empty() {
            return Err("Compilation resulted in empty native buffer");
        }

        Ok(())
    }

    /// Executes the compiled native code.
    pub unsafe fn run(&self) {
        if !self.native_code.is_empty() {
            crate::kernel::execute_jit_code(self.native_code.as_slice());
        }
    }
}