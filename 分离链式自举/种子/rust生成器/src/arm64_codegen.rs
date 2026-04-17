#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova ARM64/AArch64 代码生成助手
/// 提供 AArch64 指令的原始编码，为 Nova 多后端做准备
/// 覆盖: A64 整数/浮点/向量/系统指令

/// AArch64 通用寄存器
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reg64 { X0=0,X1,X2,X3,X4,X5,X6,X7,X8,X9,X10,X11,X12,X13,X14,X15,
                  X16,X17,X18,X19,X20,X21,X22,X23,X24,X25,X26,X27,X28,X29,X30,XZR=31 }
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reg32 { W0=0,W1,W2,W3,W4,W5,W6,W7,W8,W9,W10,W11,W12,W13,W14,W15,
                  W16,W17,W18,W19,W20,W21,W22,W23,W24,W25,W26,W27,W28,W29,W30,WZR=31 }

impl Reg64 {
    pub const SP: u8 = 31;  // SP 和 XZR 共享编号，由指令区分
    pub const LR: Reg64 = Reg64::X30;  // Link Register
    pub const FP: Reg64 = Reg64::X29;  // Frame Pointer
    pub fn idx(self) -> u32 { self as u32 }
    pub fn name(self) -> &'static str {
        const N: &[&str] = &["x0","x1","x2","x3","x4","x5","x6","x7","x8","x9","x10","x11","x12","x13","x14","x15","x16","x17","x18","x19","x20","x21","x22","x23","x24","x25","x26","x27","x28","x29","x30","xzr"];
        N[self as usize]
    }
}

/// AArch64 浮点/SIMD 寄存器
#[derive(Debug, Clone, Copy)]
pub struct VReg(pub u8);  // v0-v31

/// AArch64 指令编码器
pub struct Arm64Emitter { pub words: Vec<u32> }

impl Arm64Emitter {
    pub fn new() -> Self { Arm64Emitter { words: vec![] } }
    fn emit(&mut self, w: u32) { self.words.push(w); }
    pub fn to_bytes(&self) -> Vec<u8> {
        self.words.iter().flat_map(|&w| w.to_le_bytes()).collect()
    }

    // ─── 数据处理（寄存器）───
    // ADD Xd, Xn, Xm (moved/shifted by imm6)
    pub fn add(&mut self, rd: Reg64, rn: Reg64, rm: Reg64) {
        self.emit(0x8B000000 | rm.idx()<<16 | rn.idx()<<5 | rd.idx())
    }
    pub fn sub(&mut self, rd: Reg64, rn: Reg64, rm: Reg64) {
        self.emit(0xCB000000 | rm.idx()<<16 | rn.idx()<<5 | rd.idx())
    }
    pub fn and(&mut self, rd: Reg64, rn: Reg64, rm: Reg64) {
        self.emit(0x8A000000 | rm.idx()<<16 | rn.idx()<<5 | rd.idx())
    }
    pub fn orr(&mut self, rd: Reg64, rn: Reg64, rm: Reg64) {
        self.emit(0xAA000000 | rm.idx()<<16 | rn.idx()<<5 | rd.idx())
    }
    pub fn eor(&mut self, rd: Reg64, rn: Reg64, rm: Reg64) {
        self.emit(0xCA000000 | rm.idx()<<16 | rn.idx()<<5 | rd.idx())
    }
    pub fn mul(&mut self, rd: Reg64, rn: Reg64, rm: Reg64) {
        self.emit(0x9B007C00 | rm.idx()<<16 | rn.idx()<<5 | rd.idx())
    }
    pub fn sdiv(&mut self, rd: Reg64, rn: Reg64, rm: Reg64) {
        self.emit(0x9AC00C00 | rm.idx()<<16 | rn.idx()<<5 | rd.idx())
    }
    pub fn lsl(&mut self, rd: Reg64, rn: Reg64, rm: Reg64) {
        self.emit(0x9AC02000 | rm.idx()<<16 | rn.idx()<<5 | rd.idx())
    }
    pub fn lsr(&mut self, rd: Reg64, rn: Reg64, rm: Reg64) {
        self.emit(0x9AC02400 | rm.idx()<<16 | rn.idx()<<5 | rd.idx())
    }
    pub fn asr(&mut self, rd: Reg64, rn: Reg64, rm: Reg64) {
        self.emit(0x9AC02800 | rm.idx()<<16 | rn.idx()<<5 | rd.idx())
    }
    // CMP Xn, Xm (SUBS XZR, Xn, Xm)
    pub fn cmp(&mut self, rn: Reg64, rm: Reg64) {
        self.emit(0xEB00001F | rm.idx()<<16 | rn.idx()<<5)
    }

    // ─── 数据处理（立即数）───
    // ADD Xd, Xn, #imm12
    pub fn add_imm(&mut self, rd: Reg64, rn: Reg64, imm12: u32) {
        self.emit(0x91000000 | (imm12 & 0xFFF)<<10 | rn.idx()<<5 | rd.idx())
    }
    pub fn sub_imm(&mut self, rd: Reg64, rn: Reg64, imm12: u32) {
        self.emit(0xD1000000 | (imm12 & 0xFFF)<<10 | rn.idx()<<5 | rd.idx())
    }
    // MOV Xd, #imm16 (MOVZ)
    pub fn movz(&mut self, rd: Reg64, imm16: u16, shift: u8) {
        let hw = (shift as u32 / 16) & 3;
        self.emit(0xD2800000 | hw<<21 | (imm16 as u32)<<5 | rd.idx())
    }
    pub fn movk(&mut self, rd: Reg64, imm16: u16, shift: u8) {
        let hw = (shift as u32 / 16) & 3;
        self.emit(0xF2800000 | hw<<21 | (imm16 as u32)<<5 | rd.idx())
    }
    // MOV Xd, Xn (ORR Xd, XZR, Xn)
    pub fn mov(&mut self, rd: Reg64, rn: Reg64) {
        self.emit(0xAA0003E0 | rn.idx()<<16 | rd.idx())
    }
    // 加载64位立即数
    pub fn li64(&mut self, rd: Reg64, val: u64) {
        self.movz(rd, (val & 0xFFFF) as u16, 0);
        if val >> 16 != 0 { self.movk(rd, ((val >> 16) & 0xFFFF) as u16, 16); }
        if val >> 32 != 0 { self.movk(rd, ((val >> 32) & 0xFFFF) as u16, 32); }
        if val >> 48 != 0 { self.movk(rd, ((val >> 48) & 0xFFFF) as u16, 48); }
    }

    // ─── 加载/存储 ───
    // LDR Xd, [Xn, #offset]
    pub fn ldr(&mut self, rd: Reg64, rn: Reg64, imm9: i32) {
        self.emit(0xF8400000 | ((imm9 & 0x1FF) as u32)<<12 | rn.idx()<<5 | rd.idx())
    }
    // STR Xd, [Xn, #offset]
    pub fn str(&mut self, rd: Reg64, rn: Reg64, imm9: i32) {
        self.emit(0xF8000000 | ((imm9 & 0x1FF) as u32)<<12 | rn.idx()<<5 | rd.idx())
    }
    // LDP Xd1, Xd2, [Xn, #offset] (加载对)
    pub fn ldp(&mut self, rt1: Reg64, rt2: Reg64, rn: Reg64, imm7: i32) {
        let simm7 = ((imm7 / 8) & 0x7F) as u32;
        self.emit(0xA9400000 | simm7<<15 | rt2.idx()<<10 | rn.idx()<<5 | rt1.idx())
    }
    // STP Xd1, Xd2, [Xn, #offset] (存储对)
    pub fn stp(&mut self, rt1: Reg64, rt2: Reg64, rn: Reg64, imm7: i32) {
        let simm7 = ((imm7 / 8) & 0x7F) as u32;
        self.emit(0xA9000000 | simm7<<15 | rt2.idx()<<10 | rn.idx()<<5 | rt1.idx())
    }

    // ─── 分支 ───
    // B #offset (无条件跳转, offset 是指令数)
    pub fn b(&mut self, imm26: i32) {
        self.emit(0x14000000 | (imm26 & 0x3FFFFFF) as u32)
    }
    // BL #offset (调用)
    pub fn bl(&mut self, imm26: i32) {
        self.emit(0x94000000 | (imm26 & 0x3FFFFFF) as u32)
    }
    // BR Xn (寄存器跳转)
    pub fn br(&mut self, rn: Reg64) {
        self.emit(0xD61F0000 | rn.idx()<<5)
    }
    // BLR Xn (寄存器调用)
    pub fn blr(&mut self, rn: Reg64) {
        self.emit(0xD63F0000 | rn.idx()<<5)
    }
    // RET (返回，默认 x30)
    pub fn ret(&mut self) { self.emit(0xD65F03C0) }
    // CBZ Xn, #offset (零则跳)
    pub fn cbz(&mut self, rn: Reg64, imm19: i32) {
        self.emit(0xB4000000 | ((imm19 & 0x7FFFF) as u32)<<5 | rn.idx())
    }
    // CBNZ Xn, #offset (非零则跳)
    pub fn cbnz(&mut self, rn: Reg64, imm19: i32) {
        self.emit(0xB5000000 | ((imm19 & 0x7FFFF) as u32)<<5 | rn.idx())
    }
    // 条件分支 B.cond #offset
    pub fn b_cond(&mut self, cond: ArmCond, imm19: i32) {
        self.emit(0x54000000 | ((imm19 & 0x7FFFF) as u32)<<5 | cond as u32)
    }

    // ─── 系统 ───
    pub fn nop(&mut self)   { self.emit(0xD503201F) }
    pub fn svc(&mut self, imm16: u16) { self.emit(0xD4000001 | (imm16 as u32)<<5) }  // syscall
    pub fn brk(&mut self, imm16: u16) { self.emit(0xD4200000 | (imm16 as u32)<<5) }  // breakpoint
}

impl Default for Arm64Emitter { fn default() -> Self { Self::new() } }

/// AArch64 条件码
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum ArmCond { Eq=0, Ne=1, Cs=2, Cc=3, Mi=4, Pl=5, Vs=6, Vc=7, Hi=8, Ls=9, Ge=10, Lt=11, Gt=12, Le=13, Al=14 }

/// AArch64 调用约定
pub const ARM64_PARAM_REGS: &[Reg64] = &[Reg64::X0,Reg64::X1,Reg64::X2,Reg64::X3,Reg64::X4,Reg64::X5,Reg64::X6,Reg64::X7];
pub const ARM64_RETURN_REGS: &[Reg64] = &[Reg64::X0, Reg64::X1];
pub const ARM64_CALLEE_SAVED: &[Reg64] = &[Reg64::X19,Reg64::X20,Reg64::X21,Reg64::X22,Reg64::X23,Reg64::X24,Reg64::X25,Reg64::X26,Reg64::X27,Reg64::X28];

/// 生成函数序言 (AArch64)
pub fn emit_prologue(e: &mut Arm64Emitter, frame_size: u32) {
    // STP x29, x30, [sp, #-frame_size]!
    e.emit(0xA9B07BFD);  // stp x29, x30, [sp, #-16]!
    // MOV x29, sp
    e.emit(0x910003FD);  // add x29, sp, #0
    if frame_size > 16 {
        e.sub_imm(Reg64::X29, Reg64::X29, frame_size - 16);
    }
}

/// 生成函数尾声 (AArch64)
pub fn emit_epilogue(e: &mut Arm64Emitter) {
    e.emit(0xA8C17BFD);  // ldp x29, x30, [sp], #16
    e.ret();
}

/// macOS/Linux AArch64 syscall 号
pub const AARCH64_SYS_READ:  u64 = 63;
pub const AARCH64_SYS_WRITE: u64 = 64;
pub const AARCH64_SYS_EXIT:  u64 = 93;
pub const AARCH64_SYS_MMAP:  u64 = 222;
