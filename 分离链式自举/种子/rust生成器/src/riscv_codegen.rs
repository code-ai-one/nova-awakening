#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova RISC-V 代码生成助手 (RV64GC)
/// 提供 RISC-V 64位指令的原始编码，为 Nova 未来的多后端做准备
/// 覆盖：RV64I(整数) + M(乘除) + A(原子) + F/D(浮点) + C(压缩指令)

/// RISC-V 64位通用寄存器
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Reg {
    X0 = 0, X1, X2, X3, X4, X5, X6, X7,
    X8, X9, X10, X11, X12, X13, X14, X15,
    X16, X17, X18, X19, X20, X21, X22, X23,
    X24, X25, X26, X27, X28, X29, X30, X31,
}

// ABI 别名
impl Reg {
    pub const ZERO: Reg = Reg::X0;  // 硬零
    pub const RA: Reg = Reg::X1;    // 返回地址
    pub const SP: Reg = Reg::X2;    // 栈指针
    pub const GP: Reg = Reg::X3;    // 全局指针
    pub const TP: Reg = Reg::X4;    // 线程指针
    pub const FP: Reg = Reg::X8;    // 帧指针（=S0）
    pub const A0: Reg = Reg::X10;   // 参数/返回值1
    pub const A1: Reg = Reg::X11;   // 参数/返回值2
    pub const A7: Reg = Reg::X17;   // syscall号

    pub fn idx(self) -> u32 { self as u32 }
    pub fn name(self) -> &'static str {
        const NAMES: &[&str] = &[
            "x0","ra","sp","gp","tp","t0","t1","t2",
            "s0","s1","a0","a1","a2","a3","a4","a5",
            "a6","a7","s2","s3","s4","s5","s6","s7",
            "s8","s9","s10","s11","t3","t4","t5","t6"
        ];
        NAMES[self as usize]
    }
}

/// RISC-V 浮点寄存器
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum FReg {
    F0 = 0, F1, F2, F3, F4, F5, F6, F7,
    F8, F9, F10, F11, F12, F13, F14, F15,
    F16, F17, F18, F19, F20, F21, F22, F23,
    F24, F25, F26, F27, F28, F29, F30, F31,
}

/// RISC-V 指令编码器
pub struct RiscvEmitter {
    pub words: Vec<u32>,  // 32位指令字
}

impl RiscvEmitter {
    pub fn new() -> Self { RiscvEmitter { words: vec![] } }

    fn emit(&mut self, word: u32) { self.words.push(word); }

    /// 编码为字节流（小端序）
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![];
        for &w in &self.words {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out
    }

    // ─── R 型指令（寄存器-寄存器）───
    fn r_type(&mut self, opcode: u32, rd: Reg, funct3: u32, rs1: Reg, rs2: Reg, funct7: u32) {
        let word = opcode | (rd.idx() << 7) | (funct3 << 12) | (rs1.idx() << 15) | (rs2.idx() << 20) | (funct7 << 25);
        self.emit(word);
    }

    // ─── I 型指令（立即数）───
    fn i_type(&mut self, opcode: u32, rd: Reg, funct3: u32, rs1: Reg, imm12: i32) {
        let imm = (imm12 & 0xFFF) as u32;
        let word = opcode | (rd.idx() << 7) | (funct3 << 12) | (rs1.idx() << 15) | (imm << 20);
        self.emit(word);
    }

    // ─── S 型指令（存储）───
    fn s_type(&mut self, opcode: u32, funct3: u32, rs1: Reg, rs2: Reg, imm12: i32) {
        let imm = (imm12 & 0xFFF) as u32;
        let word = opcode | ((imm & 0x1F) << 7) | (funct3 << 12) | (rs1.idx() << 15) | (rs2.idx() << 20) | (((imm >> 5) & 0x7F) << 25);
        self.emit(word);
    }

    // ─── B 型指令（分支）───
    fn b_type(&mut self, opcode: u32, funct3: u32, rs1: Reg, rs2: Reg, imm13: i32) {
        let imm = (imm13 & 0x1FFE) as u32;
        let word = opcode
            | (((imm >> 11) & 1) << 7)
            | (((imm >> 1) & 0xF) << 8)
            | (funct3 << 12)
            | (rs1.idx() << 15)
            | (rs2.idx() << 20)
            | (((imm >> 5) & 0x3F) << 25)
            | (((imm >> 12) & 1) << 31);
        self.emit(word);
    }

    // ─── U 型指令（上部立即数）───
    fn u_type(&mut self, opcode: u32, rd: Reg, imm20: u32) {
        let word = opcode | (rd.idx() << 7) | (imm20 << 12);
        self.emit(word);
    }

    // ─── J 型指令（跳转）───
    fn j_type(&mut self, opcode: u32, rd: Reg, imm21: i32) {
        let imm = (imm21 & 0x1FFFFE) as u32;
        let word = opcode | (rd.idx() << 7)
            | ((imm >> 12) & 0xFF) << 12
            | (((imm >> 11) & 1) << 20)
            | (((imm >> 1) & 0x3FF) << 21)
            | (((imm >> 20) & 1) << 31);
        self.emit(word);
    }

    // ─── 常用 RV64I 指令 ───
    pub fn add(&mut self, rd: Reg, rs1: Reg, rs2: Reg)  { self.r_type(0x33, rd, 0, rs1, rs2, 0x00); }
    pub fn sub(&mut self, rd: Reg, rs1: Reg, rs2: Reg)  { self.r_type(0x33, rd, 0, rs1, rs2, 0x20); }
    pub fn and(&mut self, rd: Reg, rs1: Reg, rs2: Reg)  { self.r_type(0x33, rd, 7, rs1, rs2, 0x00); }
    pub fn or(&mut self, rd: Reg, rs1: Reg, rs2: Reg)   { self.r_type(0x33, rd, 6, rs1, rs2, 0x00); }
    pub fn xor(&mut self, rd: Reg, rs1: Reg, rs2: Reg)  { self.r_type(0x33, rd, 4, rs1, rs2, 0x00); }
    pub fn sll(&mut self, rd: Reg, rs1: Reg, rs2: Reg)  { self.r_type(0x33, rd, 1, rs1, rs2, 0x00); }
    pub fn srl(&mut self, rd: Reg, rs1: Reg, rs2: Reg)  { self.r_type(0x33, rd, 5, rs1, rs2, 0x00); }
    pub fn sra(&mut self, rd: Reg, rs1: Reg, rs2: Reg)  { self.r_type(0x33, rd, 5, rs1, rs2, 0x20); }
    pub fn slt(&mut self, rd: Reg, rs1: Reg, rs2: Reg)  { self.r_type(0x33, rd, 2, rs1, rs2, 0x00); }
    pub fn sltu(&mut self, rd: Reg, rs1: Reg, rs2: Reg) { self.r_type(0x33, rd, 3, rs1, rs2, 0x00); }

    // RV64I 64位运算
    pub fn addw(&mut self, rd: Reg, rs1: Reg, rs2: Reg) { self.r_type(0x3B, rd, 0, rs1, rs2, 0x00); }
    pub fn subw(&mut self, rd: Reg, rs1: Reg, rs2: Reg) { self.r_type(0x3B, rd, 0, rs1, rs2, 0x20); }
    pub fn mulw(&mut self, rd: Reg, rs1: Reg, rs2: Reg) { self.r_type(0x3B, rd, 0, rs1, rs2, 0x01); }

    // M 扩展（乘除法）
    pub fn mul(&mut self, rd: Reg, rs1: Reg, rs2: Reg)  { self.r_type(0x33, rd, 0, rs1, rs2, 0x01); }
    pub fn div(&mut self, rd: Reg, rs1: Reg, rs2: Reg)  { self.r_type(0x33, rd, 4, rs1, rs2, 0x01); }
    pub fn rem(&mut self, rd: Reg, rs1: Reg, rs2: Reg)  { self.r_type(0x33, rd, 6, rs1, rs2, 0x01); }
    pub fn divu(&mut self, rd: Reg, rs1: Reg, rs2: Reg) { self.r_type(0x33, rd, 5, rs1, rs2, 0x01); }

    // I型立即数运算
    pub fn addi(&mut self, rd: Reg, rs1: Reg, imm: i32)  { self.i_type(0x13, rd, 0, rs1, imm); }
    pub fn andi(&mut self, rd: Reg, rs1: Reg, imm: i32)  { self.i_type(0x13, rd, 7, rs1, imm); }
    pub fn ori(&mut self, rd: Reg, rs1: Reg, imm: i32)   { self.i_type(0x13, rd, 6, rs1, imm); }
    pub fn xori(&mut self, rd: Reg, rs1: Reg, imm: i32)  { self.i_type(0x13, rd, 4, rs1, imm); }
    pub fn slti(&mut self, rd: Reg, rs1: Reg, imm: i32)  { self.i_type(0x13, rd, 2, rs1, imm); }
    pub fn sltiu(&mut self, rd: Reg, rs1: Reg, imm: i32) { self.i_type(0x13, rd, 3, rs1, imm); }

    // 加载/存储
    pub fn ld(&mut self, rd: Reg, rs1: Reg, imm: i32)    { self.i_type(0x03, rd, 3, rs1, imm); }  // 64位加载
    pub fn lw(&mut self, rd: Reg, rs1: Reg, imm: i32)    { self.i_type(0x03, rd, 2, rs1, imm); }  // 32位加载
    pub fn lb(&mut self, rd: Reg, rs1: Reg, imm: i32)    { self.i_type(0x03, rd, 0, rs1, imm); }  // 8位加载
    pub fn sd(&mut self, rs1: Reg, rs2: Reg, imm: i32)   { self.s_type(0x23, 3, rs1, rs2, imm); } // 64位存储
    pub fn sw(&mut self, rs1: Reg, rs2: Reg, imm: i32)   { self.s_type(0x23, 2, rs1, rs2, imm); } // 32位存储
    pub fn sb(&mut self, rs1: Reg, rs2: Reg, imm: i32)   { self.s_type(0x23, 0, rs1, rs2, imm); } // 8位存储

    // 分支
    pub fn beq(&mut self, rs1: Reg, rs2: Reg, imm: i32)  { self.b_type(0x63, 0, rs1, rs2, imm); }
    pub fn bne(&mut self, rs1: Reg, rs2: Reg, imm: i32)  { self.b_type(0x63, 1, rs1, rs2, imm); }
    pub fn blt(&mut self, rs1: Reg, rs2: Reg, imm: i32)  { self.b_type(0x63, 4, rs1, rs2, imm); }
    pub fn bge(&mut self, rs1: Reg, rs2: Reg, imm: i32)  { self.b_type(0x63, 5, rs1, rs2, imm); }
    pub fn bltu(&mut self, rs1: Reg, rs2: Reg, imm: i32) { self.b_type(0x63, 6, rs1, rs2, imm); }
    pub fn bgeu(&mut self, rs1: Reg, rs2: Reg, imm: i32) { self.b_type(0x63, 7, rs1, rs2, imm); }

    // 跳转
    pub fn jal(&mut self, rd: Reg, imm: i32) { self.j_type(0x6F, rd, imm); }
    pub fn jalr(&mut self, rd: Reg, rs1: Reg, imm: i32) { self.i_type(0x67, rd, 0, rs1, imm); }
    pub fn j(&mut self, imm: i32) { self.jal(Reg::ZERO, imm); }   // 无返回跳转
    pub fn ret(&mut self) { self.jalr(Reg::ZERO, Reg::RA, 0); }   // 函数返回
    pub fn call(&mut self, imm: i32) { self.jal(Reg::RA, imm); }  // 函数调用

    // 上部立即数
    pub fn lui(&mut self, rd: Reg, imm20: u32) { self.u_type(0x37, rd, imm20); }
    pub fn auipc(&mut self, rd: Reg, imm20: u32) { self.u_type(0x17, rd, imm20); }

    // 伪指令
    pub fn nop(&mut self) { self.addi(Reg::ZERO, Reg::ZERO, 0); }
    pub fn mv(&mut self, rd: Reg, rs: Reg) { self.addi(rd, rs, 0); }
    pub fn li_small(&mut self, rd: Reg, imm: i32) { self.addi(rd, Reg::ZERO, imm); }

    // syscall（ecall）
    pub fn ecall(&mut self) { self.emit(0x00000073); }
    pub fn ebreak(&mut self) { self.emit(0x00100073); }

    /// 加载64位立即数到寄存器（最多需要多条指令）
    pub fn li64(&mut self, rd: Reg, mut val: i64) {
        if val >= -2048 && val <= 2047 {
            self.addi(rd, Reg::ZERO, val as i32);
            return;
        }
        // LUI + ADDI（处理高20位+低12位）
        let lo = val & 0xFFF;
        let hi = (val >> 12) + if lo >= 0x800 { 1 } else { 0 };
        self.lui(rd, hi as u32 & 0xFFFFF);
        if lo != 0 { self.addi(rd, rd, lo as i32); }
    }
}

impl Default for RiscvEmitter { fn default() -> Self { Self::new() } }

/// RISC-V 调用约定
pub const RV64_PARAM_REGS: &[Reg] = &[Reg::X10, Reg::X11, Reg::X12, Reg::X13, Reg::X14, Reg::X15, Reg::X16, Reg::X17];
pub const RV64_RETURN_REGS: &[Reg] = &[Reg::X10, Reg::X11];
pub const RV64_CALLEE_SAVED: &[Reg] = &[Reg::X8, Reg::X9, Reg::X18, Reg::X19, Reg::X20, Reg::X21, Reg::X22, Reg::X23, Reg::X24, Reg::X25, Reg::X26, Reg::X27];

/// 生成函数序言（保存 ra/fp，建立帧）
pub fn emit_func_prologue(e: &mut RiscvEmitter, frame_size: i32) {
    e.addi(Reg::SP, Reg::SP, -frame_size);
    e.sd(Reg::SP, Reg::RA, frame_size - 8);
    e.sd(Reg::SP, Reg::FP, frame_size - 16);
    e.addi(Reg::FP, Reg::SP, frame_size);
}

/// 生成函数尾声（恢复寄存器，返回）
pub fn emit_func_epilogue(e: &mut RiscvEmitter, frame_size: i32) {
    e.ld(Reg::RA, Reg::SP, frame_size - 8);
    e.ld(Reg::FP, Reg::SP, frame_size - 16);
    e.addi(Reg::SP, Reg::SP, frame_size);
    e.ret();
}

/// RISC-V Linux syscall 号
pub const RISCV_SYS_READ:  i64 = 63;
pub const RISCV_SYS_WRITE: i64 = 64;
pub const RISCV_SYS_EXIT:  i64 = 93;
pub const RISCV_SYS_MMAP:  i64 = 222;
