#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova x86-64 反汇编器（调试辅助）
/// 对生成的机器码做基本反汇编，帮助调试代码生成
/// 支持最常用的 Nova 编译器生成的指令子集

/// 反汇编指令
#[derive(Debug, Clone)]
pub struct Insn {
    pub offset: usize,
    pub bytes:  Vec<u8>,
    pub mnemonic: String,
    pub operands: String,
}

impl Insn {
    pub fn format(&self) -> String {
        let hex: String = self.bytes.iter().map(|b| format!("{:02x} ", b)).collect();
        format!("{:6x}:  {:24}  {} {}", self.offset, hex.trim_end(), self.mnemonic, self.operands)
    }
}

/// x86-64 反汇编器
pub struct Disassembler<'a> {
    code:   &'a [u8],
    offset: usize,
    base:   u64,    // 代码加载基址（用于显示绝对地址）
}

impl<'a> Disassembler<'a> {
    pub fn new(code: &'a [u8], base: u64) -> Self {
        Disassembler { code, offset: 0, base }
    }

    /// 反汇编全部代码
    pub fn disasm_all(&mut self) -> Vec<Insn> {
        let mut result = vec![];
        while self.offset < self.code.len() {
            match self.next_insn() {
                Some(insn) => result.push(insn),
                None => break,
            }
        }
        result
    }

    /// 反汇编 n 条指令
    pub fn disasm_n(&mut self, n: usize) -> Vec<Insn> {
        let mut result = vec![];
        for _ in 0..n {
            if self.offset >= self.code.len() { break; }
            match self.next_insn() {
                Some(insn) => result.push(insn),
                None => break,
            }
        }
        result
    }

    fn peek(&self) -> Option<u8> { self.code.get(self.offset).copied() }
    fn peek2(&self) -> Option<(u8, u8)> {
        Some((*self.code.get(self.offset)?, *self.code.get(self.offset + 1)?))
    }

    fn read_u8(&mut self) -> Option<u8> {
        let b = *self.code.get(self.offset)?;
        self.offset += 1;
        Some(b)
    }
    fn read_i8(&mut self) -> Option<i8> { self.read_u8().map(|b| b as i8) }
    fn read_i32(&mut self) -> Option<i32> {
        if self.offset + 4 > self.code.len() { return None; }
        let bytes = [self.code[self.offset], self.code[self.offset+1],
                     self.code[self.offset+2], self.code[self.offset+3]];
        self.offset += 4;
        Some(i32::from_le_bytes(bytes))
    }
    fn read_u32(&mut self) -> Option<u32> { self.read_i32().map(|v| v as u32) }
    fn read_u64(&mut self) -> Option<u64> {
        if self.offset + 8 > self.code.len() { return None; }
        let bytes: [u8; 8] = self.code[self.offset..self.offset+8].try_into().ok()?;
        self.offset += 8;
        Some(u64::from_le_bytes(bytes))
    }

    /// REX.W 前缀 → 64位操作数
    fn is_rex_w(rex: u8) -> bool { rex & 0x48 == 0x48 }

    fn reg64(r: u8) -> &'static str {
        match r & 7 {
            0 => "rax", 1 => "rcx", 2 => "rdx", 3 => "rbx",
            4 => "rsp", 5 => "rbp", 6 => "rsi", 7 => "rdi",
            _ => "???"
        }
    }
    fn reg32(r: u8) -> &'static str {
        match r & 7 {
            0 => "eax", 1 => "ecx", 2 => "edx", 3 => "ebx",
            4 => "esp", 5 => "ebp", 6 => "esi", 7 => "edi",
            _ => "???"
        }
    }

    fn next_insn(&mut self) -> Option<Insn> {
        let start = self.offset;
        let mut rex: u8 = 0;
        let mut has_rex = false;

        // REX前缀
        if matches!(self.peek(), Some(0x40..=0x4F)) {
            rex = self.read_u8().unwrap();
            has_rex = true;
        }

        let op = self.read_u8()?;
        let is64 = has_rex && Self::is_rex_w(rex);

        let (mnem, ops): (&'static str, String) = match op {
            // NOP
            0x90 => ("nop", String::new()),
            // PUSH reg
            0x50..=0x57 => ("push", Self::reg64(op - 0x50).into()),
            // POP reg
            0x58..=0x5F => ("pop",  Self::reg64(op - 0x58).into()),
            // RET
            0xC3 => ("ret",  String::new()),
            0xCC => ("int3", String::new()),
            // CALL rel32
            0xE8 => {
                let rel = self.read_i32()?;
                let target = (self.offset as i64 + self.base as i64 + rel as i64) as u64;
                ("call", format!("{:#x}", target))
            }
            // JMP rel32
            0xE9 => {
                let rel = self.read_i32()?;
                let target = (self.offset as i64 + self.base as i64 + rel as i64) as u64;
                ("jmp",  format!("{:#x}", target))
            }
            // JMP rel8
            0xEB => {
                let rel = self.read_i8()?;
                let target = (self.offset as i64 + self.base as i64 + rel as i64) as u64;
                ("jmp",  format!("{:#x}", target))
            }
            // JE/JNE rel8
            0x74 => { let rel = self.read_i8()?; let t = (self.offset as i64+self.base as i64+rel as i64) as u64; ("je", format!("{:#x}",t)) }
            0x75 => { let rel = self.read_i8()?; let t = (self.offset as i64+self.base as i64+rel as i64) as u64; ("jne",format!("{:#x}",t)) }
            // ADD rax, imm32
            0x05 if is64 => { let imm = self.read_i32()?; ("add", format!("rax, {}", imm)) }
            // SUB rax, imm32
            0x2D if is64 => { let imm = self.read_i32()?; ("sub", format!("rax, {}", imm)) }
            // XOR r/m64, r64 (REX.W 31 /r)
            0x31 if is64 => {
                let modrm = self.read_u8()?;
                let reg = (modrm >> 3) & 7;
                let rm = modrm & 7;
                ("xor", format!("{}, {}", Self::reg64(rm), Self::reg64(reg)))
            }
            // MOV r/m64, r64
            0x89 if is64 => {
                let modrm = self.read_u8()?;
                let reg = (modrm >> 3) & 7;
                let rm  = modrm & 7;
                if modrm & 0xC0 == 0xC0 {
                    ("mov", format!("{}, {}", Self::reg64(rm), Self::reg64(reg)))
                } else {
                    ("mov", format!("[mem], {}", Self::reg64(reg)))
                }
            }
            // MOV r64, r/m64
            0x8B if is64 => {
                let modrm = self.read_u8()?;
                let reg = (modrm >> 3) & 7;
                let rm  = modrm & 7;
                if modrm & 0xC0 == 0xC0 {
                    ("mov", format!("{}, {}", Self::reg64(reg), Self::reg64(rm)))
                } else {
                    ("mov", format!("{}, [mem]", Self::reg64(reg)))
                }
            }
            // MOV r64, imm64
            0xB8..=0xBF if is64 => {
                let reg = op - 0xB8;
                let imm = self.read_u64()?;
                ("mov", format!("{}, {:#x}", Self::reg64(reg), imm))
            }
            // MOV r32, imm32
            0xB8..=0xBF => {
                let reg = op - 0xB8;
                let imm = self.read_u32()?;
                ("mov", format!("{}, {:#x}", Self::reg32(reg), imm))
            }
            // PUSH imm32
            0x68 => { let imm = self.read_i32()?; ("push", format!("{}", imm)) }
            // TEST rax, rax
            0x85 if is64 => {
                let modrm = self.read_u8()?;
                let r = (modrm >> 3) & 7;
                let rm = modrm & 7;
                ("test", format!("{}, {}", Self::reg64(rm), Self::reg64(r)))
            }
            // 0F前缀双字节指令
            0x0F => {
                let op2 = self.read_u8()?;
                match op2 {
                    0x84 => { let rel=self.read_i32()?; let t=(self.offset as i64+self.base as i64+rel as i64) as u64; ("je",  format!("{:#x}",t)) }
                    0x85 => { let rel=self.read_i32()?; let t=(self.offset as i64+self.base as i64+rel as i64) as u64; ("jne", format!("{:#x}",t)) }
                    0x8C => { let rel=self.read_i32()?; let t=(self.offset as i64+self.base as i64+rel as i64) as u64; ("jl",  format!("{:#x}",t)) }
                    0x8F => { let rel=self.read_i32()?; let t=(self.offset as i64+self.base as i64+rel as i64) as u64; ("jg",  format!("{:#x}",t)) }
                    0x05 => ("syscall", String::new()),
                    _ => {
                        ("?0f", format!("{:02x}", op2))
                    }
                }
            }
            // SYSCALL (bare)
            _ => {
                let bytes: Vec<u8> = self.code[start..self.offset].to_vec();
                ("db", bytes.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "))
            }
        };

        let bytes: Vec<u8> = self.code[start..self.offset].to_vec();
        Some(Insn {
            offset: start + self.base as usize,
            bytes,
            mnemonic: mnem.into(),
            operands: ops,
        })
    }
}

/// 反汇编并格式化输出字符串
pub fn disasm_to_string(code: &[u8], base: u64, max_insns: usize) -> String {
    let mut d = Disassembler::new(code, base);
    let insns = d.disasm_n(max_insns);
    let mut out = String::new();
    for insn in &insns {
        out.push_str(&insn.format());
        out.push('\n');
    }
    out
}

/// 对函数代码段做基本分析（统计指令类型分布）
pub fn analyze_code_profile(code: &[u8]) -> CodeProfile {
    let mut d = Disassembler::new(code, 0);
    let insns = d.disasm_all();
    let mut profile = CodeProfile::default();
    profile.total_insns = insns.len();
    for insn in &insns {
        match insn.mnemonic.as_str() {
            "call" => profile.calls += 1,
            "ret"  => profile.rets += 1,
            "jmp" | "je" | "jne" | "jl" | "jg" | "jle" | "jge" => profile.jumps += 1,
            "mov"  => profile.movs += 1,
            "add" | "sub" | "imul" | "idiv" => profile.arith += 1,
            "push" | "pop" => profile.stack_ops += 1,
            _ => {}
        }
    }
    profile
}

#[derive(Debug, Default)]
pub struct CodeProfile {
    pub total_insns: usize,
    pub calls:      usize,
    pub rets:       usize,
    pub jumps:      usize,
    pub movs:       usize,
    pub arith:      usize,
    pub stack_ops:  usize,
}

impl CodeProfile {
    pub fn format(&self) -> String {
        format!(
            "总指令: {} | call: {} | ret: {} | 跳转: {} | mov: {} | 算术: {} | 栈操作: {}",
            self.total_insns, self.calls, self.rets, self.jumps,
            self.movs, self.arith, self.stack_ops
        )
    }
}
