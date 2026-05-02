//! NebulaOS JIT Compiler IR and Engine.
//! This module handles the translation of architecture-independent bytecode into native machine code.

use alloc::vec::Vec;

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
    /// Divide two registers: VReg(Dst) = VReg(Src1) / VReg(Src2) (unsigned)
    Div(VReg, VReg, VReg),
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
    /// Add two floating-point registers: VReg(Dst) = VReg(Src1) + VReg(Src2)
    FAdd(VReg, VReg, VReg),
    /// Subtract two floating-point registers: VReg(Dst) = VReg(Src1) - VReg(Src2)
    FSub(VReg, VReg, VReg),
    /// Multiply two floating-point registers: VReg(Dst) = VReg(Src1) * VReg(Src2)
    FMul(VReg, VReg, VReg),
    /// Divide two floating-point registers: VReg(Dst) = VReg(Src1) / VReg(Src2)
    FDiv(VReg, VReg, VReg),
    /// Add two double-precision registers: VReg(Dst) = VReg(Src1) + VReg(Src2)
    DAdd(VReg, VReg, VReg),
    /// Subtract two double-precision registers
    DSub(VReg, VReg, VReg),
    /// Multiply two double-precision registers
    DMul(VReg, VReg, VReg),
    /// Divide two double-precision registers
    DDiv(VReg, VReg, VReg),
    /// Vector Add (Packed Single/Double): VReg(Dst) = VReg(Src1) + VReg(Src2)
    VAdd(VReg, VReg, VReg),
    /// Vector Subtract
    VSub(VReg, VReg, VReg),
    /// Vector Multiply
    VMul(VReg, VReg, VReg),
    /// Vector Divide
    VDiv(VReg, VReg, VReg),
    /// Vector Bitwise XOR
    VXor(VReg, VReg, VReg),
    /// Vector Bitwise AND
    VAnd(VReg, VReg, VReg),
    /// Vector Bitwise OR
    VOr(VReg, VReg, VReg),
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

    /// Calculates the native byte size of an IR instruction for the current architecture.
    fn instr_size(&self, instr: &Instruction) -> usize {
        #[cfg(target_arch = "x86_64")]
        {
            return match instr {
                Instruction::Ret => 1,
                Instruction::LoadImm(_, _) => 10,
                Instruction::Load(_, _) | Instruction::Store(_, _) => 3,
                Instruction::Mov(dst, src) => if dst.0 == src.0 { 0 } else { 3 },
                Instruction::Div(_, _, _) => 12, // Max size: MOV RAX, src1 (3) + XOR RDX, RDX (3) + DIV src2 (3) + MOV dst, RAX (3)
                Instruction::Add(dst, src1, _) | Instruction::Sub(dst, src1, _) | 
                Instruction::And(dst, src1, _) | Instruction::Or(dst, src1, _) | 
                Instruction::Xor(dst, src1, _) => {
                    let base = 3;
                    if dst.0 != src1.0 { base + 3 } else { base }
                }
                Instruction::Mul(dst, src1, _) => {
                    let base = 4;
                    if dst.0 != src1.0 { base + 3 } else { base }
                }
                Instruction::Cmp(_, _) => 3,
                Instruction::Jmp(_) => 5,
                Instruction::JmpEq(_) | Instruction::JmpNe(_) | Instruction::JmpLt(_) |
                Instruction::JmpGt(_) | Instruction::JmpGe(_) | Instruction::JmpLe(_) => 6,
                Instruction::SysCall(_) => 7,
                Instruction::Shl(_, _, _) | Instruction::Shr(_, _, _) => 4, // REX.W + D3 /4 or /5 + ModR/M
                Instruction::FAdd(_, _, _) | Instruction::FSub(_, _, _) | Instruction::FMul(_, _, _) | Instruction::FDiv(_, _, _) |
                Instruction::DAdd(_, _, _) | Instruction::DSub(_, _, _) | Instruction::DMul(_, _, _) | Instruction::DDiv(_, _, _) |
                Instruction::VAdd(_, _, _) | Instruction::VSub(_, _, _) | Instruction::VMul(_, _, _) | Instruction::VDiv(_, _, _) |
                Instruction::VXor(_, _, _) | Instruction::VAnd(_, _, _) | Instruction::VOr(_, _, _) => 6, // Prefix + REX + 0F + Op + ModRM
                _ => 0,
            };
        }
        #[cfg(target_arch = "x86")]
        {
            return match instr {
                Instruction::Ret => 1,
                Instruction::LoadImm(_, _) => 5,
                Instruction::Load(_, _) | Instruction::Store(_, _) => 2,
                Instruction::Mov(dst, src) => if dst.0 == src.0 { 0 } else { 2 },
                Instruction::Add(dst, src1, _) | Instruction::Sub(dst, src1, _) | 
                Instruction::And(dst, src1, _) | Instruction::Or(dst, src1, _) | 
                Instruction::Xor(dst, src1, _) => {
                    let base = 2;
                    if dst.0 != src1.0 { base + 2 } else { base }
                }
                Instruction::Mul(dst, src1, _) => {
                    let base = 3;
                    if dst.0 != src1.0 { base + 2 } else { base }
                }
                Instruction::Cmp(_, _) => 2,
                Instruction::Jmp(_) => 5,
                Instruction::JmpEq(_) | Instruction::JmpNe(_) | Instruction::JmpLt(_) |
                Instruction::JmpGt(_) | Instruction::JmpGe(_) | Instruction::JmpLe(_) => 6,
                Instruction::SysCall(_) => 7,
                Instruction::Div(_, _, _) => 8, // Max size for x86: MOV EAX, src1 (2) + XOR EDX, EDX (2) + DIV src2 (2) + MOV dst, EAX (2)
                Instruction::Shl(_, _, _) | Instruction::Shr(_, _, _) => 4, // MOV src, dst (2) + SHL/SHR (2)
                Instruction::FAdd(_, _, _) | Instruction::FSub(_, _, _) | Instruction::FMul(_, _, _) | Instruction::FDiv(_, _, _) |
                Instruction::DAdd(_, _, _) | Instruction::DSub(_, _, _) | Instruction::DMul(_, _, _) | Instruction::DDiv(_, _, _) |
                Instruction::VAdd(_, _, _) | Instruction::VSub(_, _, _) | Instruction::VMul(_, _, _) | Instruction::VDiv(_, _, _) |
                Instruction::VXor(_, _, _) | Instruction::VAnd(_, _, _) | Instruction::VOr(_, _, _) => 5,
            };
        }
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        0
    }

    /// Maps a virtual register to a safe physical register index,
    /// ensuring we avoid restricted registers like RSP (4) and RBP (5).
    fn get_phys_reg(&self, vreg: VReg) -> u8 {
        #[cfg(target_arch = "x86_64")]
        {
            // Available: RAX(0), RCX(1), RDX(2), RBX(3), RSI(6), RDI(7), R8-R15(8-15)
            let safe_regs: [u8; 14] = [0, 1, 2, 3, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
            return safe_regs[(vreg.0 as usize) % safe_regs.len()];
        }
        #[cfg(target_arch = "x86")]
        {
            // Available: EAX(0), ECX(1), EDX(2), EBX(3), ESI(6), EDI(7)
            let safe_regs: [u8; 6] = [0, 1, 2, 3, 6, 7];
            return safe_regs[(vreg.0 as usize) % safe_regs.len()];
        }
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        0
    }

    /// Maps a virtual register to a physical XMM register index (0-7 or 0-15).
    fn get_phys_freg(&self, vreg: VReg) -> u8 {
        #[cfg(target_arch = "x86_64")]
        { return vreg.0 % 16; }
        #[cfg(target_arch = "x86")]
        { return vreg.0 % 8; }
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        0
    }

    /// Basic translation stub. 
    /// This will eventually iterate through ir_code and emit x86/x86_64 opcodes.
    pub fn compile(&mut self) -> Result<(), &'static str> {
        self.native_code.clear();

        // PASS 1: Calculate byte offsets for every IR instruction
        let mut instr_offsets = alloc::vec![0usize; self.ir_code.len() + 1];
        for i in 0..self.ir_code.len() {
            instr_offsets[i + 1] = instr_offsets[i] + self.instr_size(&self.ir_code[i]);
        }

        // PASS 2: Emit native machine code
        for i in 0..self.ir_code.len() {
            let instr = &self.ir_code[i];
            let next_instr_addr = instr_offsets[i + 1];

            match instr {
                Instruction::Ret => {
                    // Native x86 'ret' opcode
                    self.native_code.push(0xC3);
                }
                Instruction::LoadImm(reg, val) => {
                    #[cfg(target_arch = "x86")]
                    {
                        // MOV reg, imm32
                        let p_reg = self.get_phys_reg(*reg);
                        // Opcode 0xB8 + reg_index
                        self.native_code.push(0xB8 + p_reg);
                        let bytes = (*val as u32).to_le_bytes();
                        self.native_code.extend_from_slice(&bytes);
                    }
                    #[cfg(target_arch = "x86_64")]
                    {
                        // MOVABS rax, imm64 (Standard 64-bit load)
                        let p_reg = self.get_phys_reg(*reg);
                        // REX.W + 0xB8 + reg_index
                        self.native_code.push(0x48);
                        self.native_code.push(0xB8 + p_reg);
                        let bytes = (*val as u64).to_le_bytes();
                        self.native_code.extend_from_slice(&bytes);
                    }
                }
                Instruction::Load(_dst, _addr) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        // MOV r64, [r64] -> REX.W + 0x8B + ModR/M
                        let p_dst = self.get_phys_reg(*_dst);
                        let p_addr = self.get_phys_reg(*_addr);
                        let mut rex = 0x48;
                        if p_dst > 7 { rex |= 0x04; } // REX.R (dst)
                        if p_addr > 7 { rex |= 0x01; } // REX.B (base addr)
                        self.native_code.push(rex);
                        self.native_code.push(0x8B);
                        self.native_code.push(((p_dst & 7) << 3) | (p_addr & 7));
                    }
                }
                Instruction::Store(_addr, _src) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        // MOV [r64], r64 -> REX.W + 0x89 + ModR/M
                        let p_src = self.get_phys_reg(*_src);
                        let p_addr = self.get_phys_reg(*_addr);
                        let mut rex = 0x48;
                        if p_src > 7 { rex |= 0x04; } 
                        if p_addr > 7 { rex |= 0x01; } // REX.B (base addr)
                        self.native_code.push(rex);
                        self.native_code.push(0x89);
                        self.native_code.push(((p_src & 7) << 3) | (p_addr & 7));
                    }
                }
                Instruction::Mov(_dst, _src) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_dst = self.get_phys_reg(*_dst);
                        let p_src = self.get_phys_reg(*_src);
                        if p_dst != p_src {
                            // MOV r/m64, r64 -> REX.W + 0x89 + ModR/M
                            let mut rex = 0x48; // REX.W
                            if p_src > 7 { rex |= 0x04; }
                            if p_dst > 7 { rex |= 0x01; }
                            self.native_code.push(rex);
                            self.native_code.push(0x89);
                            self.native_code.push(0xC0 | ((p_src & 7) << 3) | (p_dst & 7));
                        }
                    }
                }
                Instruction::Add(_dst, _src1, _src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_dst = self.get_phys_reg(*_dst);
                        let p_src1 = self.get_phys_reg(*_src1);
                        let p_src2 = self.get_phys_reg(*_src2);

                        // 1. Ensure dst contains src1: MOV dst, src1
                        if p_dst != p_src1 {
                            let mut rex = 0x48;
                            if p_src1 > 7 { rex |= 0x04; }
                            if p_dst > 7 { rex |= 0x01; }
                            self.native_code.push(rex);
                            self.native_code.push(0x89);
                            self.native_code.push(0xC0 | ((p_src1 & 7) << 3) | (p_dst & 7));
                        }
                        // 2. Perform addition: ADD dst, src2
                        let mut rex = 0x48;
                        if p_src2 > 7 { rex |= 0x04; }
                        if p_dst > 7 { rex |= 0x01; }
                        self.native_code.push(rex);
                        self.native_code.push(0x01);
                        self.native_code.push(0xC0 | ((p_src2 & 7) << 3) | (p_dst & 7));
                    }
                }
                Instruction::Sub(_dst, _src1, _src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_dst = self.get_phys_reg(*_dst);
                        let p_src1 = self.get_phys_reg(*_src1);
                        let p_src2 = self.get_phys_reg(*_src2);

                        // 1. Ensure dst contains src1: MOV dst, src1
                        if p_dst != p_src1 {
                            let mut rex = 0x48;
                            if p_src1 > 7 { rex |= 0x04; }
                            if p_dst > 7 { rex |= 0x01; }
                            self.native_code.push(rex);
                            self.native_code.push(0x89);
                            self.native_code.push(0xC0 | ((p_src1 & 7) << 3) | (p_dst & 7));
                        }
                        // 2. Perform subtraction: SUB dst, src2
                        let mut rex = 0x48;
                        if p_src2 > 7 { rex |= 0x04; }
                        if p_dst > 7 { rex |= 0x01; }
                        self.native_code.push(rex);
                        self.native_code.push(0x29);
                        self.native_code.push(0xC0 | ((p_src2 & 7) << 3) | (p_dst & 7));
                    }
                }
                Instruction::Mul(_dst, _src1, _src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_dst = self.get_phys_reg(*_dst);
                        let p_src1 = self.get_phys_reg(*_src1);
                        let p_src2 = self.get_phys_reg(*_src2);

                        if p_dst != p_src1 {
                            let mut rex = 0x48;
                            if p_src1 > 7 { rex |= 0x04; }
                            if p_dst > 7 { rex |= 0x01; }
                            self.native_code.push(rex);
                            self.native_code.push(0x89);
                            self.native_code.push(0xC0 | ((p_src1 & 7) << 3) | (p_dst & 7));
                        }
                        // IMUL r64, r/m64 -> REX.W + 0x0F 0xAF + ModR/M
                        self.native_code.push(0x48);
                        self.native_code.push(0x0F);
                        self.native_code.push(0xAF);
                        self.native_code.push(0xC0 | ((p_src2 & 7) << 3) | (p_dst & 7));
                    }
                }
                Instruction::Div(_dst, _src1, _src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_dst = self.get_phys_reg(*_dst);
                        let p_src1 = self.get_phys_reg(*_src1);
                        let p_src2 = self.get_phys_reg(*_src2);

                        // 1. Move src1 (dividend) to RAX (physical register 0)
                        // MOV RAX, p_src1
                        if p_src1 != 0 { // If src1 is not already RAX
                            let mut rex = 0x48; // REX.W
                            if p_src1 > 7 { rex |= 0x04; } // REX.R bit (for src)
                            self.native_code.push(rex);
                            self.native_code.push(0x89); // MOV r/m64, r64
                            self.native_code.push(0xC0 | ((p_src1 & 7) << 3) | (0 & 7)); // ModR/M: Mod=11 (reg), Reg=p_src1, R/M=RAX
                        }

                        // 2. Zero RDX (physical register 2) for unsigned 128-bit dividend RDX:RAX
                        // XOR RDX, RDX (3 bytes: 0x48 0x31 0xD2)
                        self.native_code.extend_from_slice(&[0x48, 0x31, 0xD2]);

                        // 3. Perform DIV src2_phys_reg
                        // DIV r/m64 -> REX.W + 0xF7 /6 + ModR/M
                        let mut rex = 0x48; // REX.W
                        if p_src2 > 7 { rex |= 0x01; } // REX.B bit (for r/m operand)
                        self.native_code.push(rex);
                        self.native_code.push(0xF7);
                        self.native_code.push(0xF0 | (p_src2 & 7)); // ModR/M: Mod=11 (reg), Reg=6 (DIV), R/M=p_src2

                        // 4. Move result (quotient from RAX) to dst
                        // MOV p_dst, RAX
                        if p_dst != 0 { // If dst is not already RAX
                            let mut rex = 0x48; // REX.W
                            if p_dst > 7 { rex |= 0x01; } // REX.B bit (for dst)
                            self.native_code.push(rex);
                            self.native_code.push(0x89); // MOV r/m64, r64
                            self.native_code.push(0xC0 | ((0 & 7) << 3) | (p_dst & 7)); // ModR/M: Mod=11 (reg), Reg=RAX, R/M=p_dst
                        }
                    }
                    #[cfg(target_arch = "x86")]
                    {
                        let p_dst = self.get_phys_reg(*_dst);
                        let p_src1 = self.get_phys_reg(*_src1);
                        let p_src2 = self.get_phys_reg(*_src2);

                        // 1. Move src1 (dividend) to EAX (physical register 0)
                        if p_src1 != 0 {
                            self.native_code.push(0x8B); 
                            self.native_code.push(0xC0 | ((p_src1 & 7) << 3) | 0);
                        }
                        // 2. Zero EDX
                        self.native_code.extend_from_slice(&[0x31, 0xD2]);
                        // 3. Perform DIV
                        self.native_code.push(0xF7);
                        self.native_code.push(0xF0 | (p_src2 & 7));
                        // 4. Move result to dst
                        if p_dst != 0 {
                            self.native_code.push(0x89);
                            self.native_code.push(0xC0 | (0 << 3) | (p_dst & 7));
                        }
                    }
                }
                Instruction::And(_dst, _src1, _src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_dst = self.get_phys_reg(*_dst);
                        let p_src1 = self.get_phys_reg(*_src1);
                        let p_src2 = self.get_phys_reg(*_src2);

                        if p_dst != p_src1 { self.native_code.extend_from_slice(&[0x48, 0x89, 0xC0 | ((p_src1 & 7) << 3) | (p_dst & 7)]); }
                        self.native_code.extend_from_slice(&[0x48, 0x21, 0xC0 | ((p_src2 & 7) << 3) | (p_dst & 7)]);
                    }
                }
                Instruction::Or(_dst, _src1, _src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_dst = self.get_phys_reg(*_dst);
                        let p_src1 = self.get_phys_reg(*_src1);
                        let p_src2 = self.get_phys_reg(*_src2);

                        if p_dst != p_src1 { self.native_code.extend_from_slice(&[0x48, 0x89, 0xC0 | ((p_src1 & 7) << 3) | (p_dst & 7)]); }
                        self.native_code.extend_from_slice(&[0x48, 0x09, 0xC0 | ((p_src2 & 7) << 3) | (p_dst & 7)]);
                    }
                }
                Instruction::Xor(_dst, _src1, _src2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_dst = self.get_phys_reg(*_dst);
                        let p_src1 = self.get_phys_reg(*_src1);
                        let p_src2 = self.get_phys_reg(*_src2);

                        if p_dst != p_src1 { self.native_code.extend_from_slice(&[0x48, 0x89, 0xC0 | ((p_src1 & 7) << 3) | (p_dst & 7)]); }
                        self.native_code.extend_from_slice(&[0x48, 0x31, 0xC0 | ((p_src2 & 7) << 3) | (p_dst & 7)]);
                    }
                }
                Instruction::Shl(_dst, _src, _count) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_dst = self.get_phys_reg(*_dst);
                        let p_src = self.get_phys_reg(*_src);
                        let _p_count = self.get_phys_reg(*_count); 

                        if p_dst != p_src { self.native_code.extend_from_slice(&[0x48, 0x89, 0xC0 | ((p_src & 7) << 3) | (p_dst & 7)]); }
                        self.native_code.extend_from_slice(&[0x48, 0xD3, 0xE0 | (p_dst & 7)]); // SHL R/M64, CL
                    }
                }
                Instruction::Shr(_dst, _src, _count) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_dst = self.get_phys_reg(*_dst);
                        let p_src = self.get_phys_reg(*_src);
                        let _p_count = self.get_phys_reg(*_count);

                        if p_dst != p_src { self.native_code.extend_from_slice(&[0x48, 0x89, 0xC0 | ((p_src & 7) << 3) | (p_dst & 7)]); }
                        self.native_code.extend_from_slice(&[0x48, 0xD3, 0xE8 | (p_dst & 7)]); // SHR R/M64, CL
                    }
                }
                Instruction::FAdd(_d, _s1, _s2) | Instruction::FSub(_d, _s1, _s2) | 
                Instruction::FMul(_d, _s1, _s2) | Instruction::FDiv(_d, _s1, _s2) |
                Instruction::DAdd(_d, _s1, _s2) | Instruction::DSub(_d, _s1, _s2) | 
                Instruction::DMul(_d, _s1, _s2) | Instruction::DDiv(_d, _s1, _s2) |
                Instruction::VAdd(_d, _s1, _s2) | Instruction::VSub(_d, _s1, _s2) | 
                Instruction::VMul(_d, _s1, _s2) | Instruction::VDiv(_d, _s1, _s2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_dst = self.get_phys_freg(*_d);
                        let p_s1 = self.get_phys_freg(*_s1);
                        let p_s2 = self.get_phys_freg(*_s2);
                        
                        let (prefix, opcode) = match instr {
                            Instruction::FAdd(..) => (Some(0xF3), 0x58),
                            Instruction::FSub(..) => (Some(0xF3), 0x5C),
                            Instruction::FMul(..) => (Some(0xF3), 0x59),
                            Instruction::FDiv(..) => (Some(0xF3), 0x5E),
                            Instruction::DAdd(..) => (Some(0xF2), 0x58),
                            Instruction::DSub(..) => (Some(0xF2), 0x5C),
                            Instruction::DMul(..) => (Some(0xF2), 0x59),
                            Instruction::DDiv(..) => (Some(0xF2), 0x5E),
                            Instruction::VAdd(..) => (None, 0x58),
                            Instruction::VSub(..) => (None, 0x5C),
                            Instruction::VMul(..) => (None, 0x59),
                            Instruction::VDiv(..) => (None, 0x5E),
                            _ => (None, 0x00),
                        };

                        // 1. Move s1 to dst if needed
                        if p_dst != p_s1 {
                            if let Some(p) = prefix { self.native_code.push(p); }
                            let mut rex = 0x40;
                            if p_dst > 7 { rex |= 0x04; }
                            if p_s1 > 7 { rex |= 0x01; }
                            if rex > 0x40 { self.native_code.push(rex); }
                            self.native_code.extend_from_slice(&[0x0F, 0x10, 0xC0 | ((p_dst & 7) << 3) | (p_s1 & 7)]);
                        }
                        
                        // 2. Perform arithmetic
                        if let Some(p) = prefix { self.native_code.push(p); }
                        let mut rex = 0x40;
                        if p_dst > 7 { rex |= 0x04; }
                        if p_s2 > 7 { rex |= 0x01; }
                        if rex > 0x40 { self.native_code.push(rex); }
                        self.native_code.extend_from_slice(&[0x0F, opcode, 0xC0 | ((p_dst & 7) << 3) | (p_s2 & 7)]);
                    }
                }
                Instruction::VXor(_d, _s1, _s2) | Instruction::VAnd(_d, _s1, _s2) | Instruction::VOr(_d, _s1, _s2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_dst = self.get_phys_freg(*_d);
                        let p_s1 = self.get_phys_freg(*_s1);
                        let p_s2 = self.get_phys_freg(*_s2);
                        
                        let opcode = match instr {
                            Instruction::VXor(..) => 0x57, // XORPS
                            Instruction::VAnd(..) => 0x54, // ANDPS
                            Instruction::VOr(..)  => 0x56, // ORPS
                            _ => 0x00,
                        };

                        if p_dst != p_s1 {
                            let mut rex = 0x40;
                            if p_dst > 7 { rex |= 0x04; }
                            if p_s1 > 7 { rex |= 0x01; }
                            if rex > 0x40 { self.native_code.push(rex); }
                            self.native_code.extend_from_slice(&[0x0F, 0x10, 0xC0 | ((p_dst & 7) << 3) | (p_s1 & 7)]);
                        }
                        
                        let mut rex = 0x40;
                        if p_dst > 7 { rex |= 0x04; }
                        if p_s2 > 7 { rex |= 0x01; }
                        if rex > 0x40 { self.native_code.push(rex); }
                        self.native_code.extend_from_slice(&[0x0F, opcode, 0xC0 | ((p_dst & 7) << 3) | (p_s2 & 7)]);
                    }
                }
                Instruction::Cmp(v1, v2) => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        let p_v1 = self.get_phys_reg(*v1);
                        let p_v2 = self.get_phys_reg(*v2);
                        self.native_code.extend_from_slice(&[0x48, 0x39, 0xC0 | ((p_v2 & 7) << 3) | (p_v1 & 7)]);
                    }
                    #[cfg(target_arch = "x86")]
                    {
                        // CMP r/m32, r32 -> 0x39 + ModR/M
                        let p_v1 = self.get_phys_reg(*v1);
                        let p_v2 = self.get_phys_reg(*v2);
                        self.native_code.push(0x39);
                        self.native_code.push(0xC0 | ((p_v2 & 7) << 3) | (p_v1 & 7));
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
            unsafe {
                crate::kernel::execute_jit_code(self.native_code.as_slice());
            }
        }
    }
}