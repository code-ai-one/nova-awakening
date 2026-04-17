#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova SIMD 指令发射助手 (x86-64 SSE/AVX)
/// 为 Nova 代码生成器提供向量化指令的原始字节序列
/// 覆盖: 整数/浮点向量运算, 内存对齐, 广播/洗牌

/// SIMD 寄存器类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimdReg {
    Xmm(u8),   // 128位 (0-15)
    Ymm(u8),   // 256位 (0-15, 需要 AVX)
    Zmm(u8),   // 512位 (0-31, 需要 AVX-512)
}

impl SimdReg {
    pub fn idx(self) -> u8 {
        match self { SimdReg::Xmm(i) | SimdReg::Ymm(i) | SimdReg::Zmm(i) => i }
    }
    pub fn bits(self) -> usize {
        match self { SimdReg::Xmm(_) => 128, SimdReg::Ymm(_) => 256, SimdReg::Zmm(_) => 512 }
    }
    pub fn elements_i32(self) -> usize { self.bits() / 32 }
    pub fn elements_f64(self) -> usize { self.bits() / 64 }
}

/// SIMD 元素类型
#[derive(Debug, Clone, Copy)]
pub enum SimdElem {
    I8, I16, I32, I64,
    F32, F64,
}

impl SimdElem {
    pub fn bytes(self) -> usize {
        match self {
            SimdElem::I8 | SimdElem::F32 /* f32=4B */ => match self {
                SimdElem::I8 => 1, SimdElem::F32 => 4, _ => unreachable!()
            },
            SimdElem::I16 => 2,
            SimdElem::I32 => 4,
            SimdElem::I64 | SimdElem::F64 => 8,
        }
    }
}

/// SIMD 指令构建器
pub struct SimdEmitter {
    pub bytes: Vec<u8>,
}

impl SimdEmitter {
    pub fn new() -> Self { SimdEmitter { bytes: Vec::new() } }
    pub fn clear(&mut self) { self.bytes.clear(); }

    fn emit(&mut self, b: u8) { self.bytes.push(b); }
    fn emit2(&mut self, a: u8, b: u8) { self.bytes.push(a); self.bytes.push(b); }
    fn emit3(&mut self, a: u8, b: u8, c: u8) { self.bytes.extend_from_slice(&[a, b, c]); }
    fn emit4(&mut self, a: u8, b: u8, c: u8, d: u8) { self.bytes.extend_from_slice(&[a, b, c, d]); }

    // ModRM 字节：mod=11(reg-reg), reg, rm
    fn modrm_rr(reg: u8, rm: u8) -> u8 { 0xC0 | ((reg & 7) << 3) | (rm & 7) }

    // ─── SSE/SSE2 指令 ───

    /// MOVDQU xmm, [rsp+offset]  — 非对齐加载128位
    pub fn movdqu_load(&mut self, dst: SimdReg, rsp_offset: i32) {
        let r = dst.idx();
        // F3 0F 6F /r  MOVDQU xmm, m128
        self.emit3(0xF3, 0x0F, 0x6F);
        if rsp_offset == 0 {
            self.emit(0x04 | ((r & 7) << 3));  // ModRM: [rsp]
            self.emit(0x24);  // SIB: [rsp]
        } else if rsp_offset >= -128 && rsp_offset <= 127 {
            self.emit(0x44 | ((r & 7) << 3));
            self.emit(0x24);
            self.emit(rsp_offset as u8);
        } else {
            self.emit(0x84 | ((r & 7) << 3));
            self.emit(0x24);
            self.bytes.extend_from_slice(&(rsp_offset as i32).to_le_bytes());
        }
    }

    /// MOVDQU [rsp+offset], xmm  — 非对齐存储128位
    pub fn movdqu_store(&mut self, rsp_offset: i32, src: SimdReg) {
        let r = src.idx();
        // F3 0F 7F /r  MOVDQU m128, xmm
        self.emit3(0xF3, 0x0F, 0x7F);
        if rsp_offset == 0 {
            self.emit(0x04 | ((r & 7) << 3));
            self.emit(0x24);
        } else if rsp_offset >= -128 && rsp_offset <= 127 {
            self.emit(0x44 | ((r & 7) << 3));
            self.emit(0x24);
            self.emit(rsp_offset as u8);
        } else {
            self.emit(0x84 | ((r & 7) << 3));
            self.emit(0x24);
            self.bytes.extend_from_slice(&(rsp_offset as i32).to_le_bytes());
        }
    }

    /// PADDD xmm1, xmm2  — 32位整数向量加法
    pub fn paddd(&mut self, dst: SimdReg, src: SimdReg) {
        // 66 0F FE /r
        self.emit4(0x66, 0x0F, 0xFE, Self::modrm_rr(dst.idx(), src.idx()));
    }

    /// PSUBD xmm1, xmm2  — 32位整数向量减法
    pub fn psubd(&mut self, dst: SimdReg, src: SimdReg) {
        // 66 0F FA /r
        self.emit4(0x66, 0x0F, 0xFA, Self::modrm_rr(dst.idx(), src.idx()));
    }

    /// PMULLD xmm1, xmm2  — 32位整数向量乘法（低32位）(需SSE4.1)
    pub fn pmulld(&mut self, dst: SimdReg, src: SimdReg) {
        // 66 0F 38 40 /r
        self.bytes.extend_from_slice(&[0x66, 0x0F, 0x38, 0x40]);
        self.emit(Self::modrm_rr(dst.idx(), src.idx()));
    }

    /// ADDPS xmm1, xmm2  — 单精度浮点向量加法
    pub fn addps(&mut self, dst: SimdReg, src: SimdReg) {
        // 0F 58 /r
        self.emit3(0x0F, 0x58, Self::modrm_rr(dst.idx(), src.idx()));
    }

    /// MULPS xmm1, xmm2  — 单精度浮点向量乘法
    pub fn mulps(&mut self, dst: SimdReg, src: SimdReg) {
        // 0F 59 /r
        self.emit3(0x0F, 0x59, Self::modrm_rr(dst.idx(), src.idx()));
    }

    /// ADDPD xmm1, xmm2  — 双精度浮点向量加法
    pub fn addpd(&mut self, dst: SimdReg, src: SimdReg) {
        // 66 0F 58 /r
        self.emit4(0x66, 0x0F, 0x58, Self::modrm_rr(dst.idx(), src.idx()));
    }

    /// MULPD xmm1, xmm2  — 双精度浮点向量乘法
    pub fn mulpd(&mut self, dst: SimdReg, src: SimdReg) {
        // 66 0F 59 /r
        self.emit4(0x66, 0x0F, 0x59, Self::modrm_rr(dst.idx(), src.idx()));
    }

    /// PXOR xmm1, xmm2  — 向量位异或（常用于清零）
    pub fn pxor(&mut self, dst: SimdReg, src: SimdReg) {
        // 66 0F EF /r
        self.emit4(0x66, 0x0F, 0xEF, Self::modrm_rr(dst.idx(), src.idx()));
    }

    /// 将 xmm 寄存器清零: PXOR xmm, xmm
    pub fn zero_xmm(&mut self, reg: SimdReg) { self.pxor(reg, reg); }

    /// PSHUFD xmm1, xmm2/m128, imm8  — 双字广播/洗牌
    pub fn pshufd(&mut self, dst: SimdReg, src: SimdReg, imm: u8) {
        // 66 0F 70 /r ib
        self.bytes.extend_from_slice(&[0x66, 0x0F, 0x70,
            Self::modrm_rr(dst.idx(), src.idx()), imm]);
    }

    /// 广播 xmm[0] 到所有4个32位通道: PSHUFD xmm, xmm, 0x00
    pub fn broadcast_i32(&mut self, reg: SimdReg) { self.pshufd(reg, reg, 0x00); }

    /// PCMPEQD xmm1, xmm2  — 32位整数向量比较（等于）→ 全1或全0
    pub fn pcmpeqd(&mut self, dst: SimdReg, src: SimdReg) {
        // 66 0F 76 /r
        self.emit4(0x66, 0x0F, 0x76, Self::modrm_rr(dst.idx(), src.idx()));
    }

    /// MOVMSKPS eax, xmm  — 提取浮点符号位到通用寄存器
    pub fn movmskps(&mut self, dst_gpr: u8, src: SimdReg) {
        // 0F 50 /r
        self.emit3(0x0F, 0x50, Self::modrm_rr(dst_gpr, src.idx()));
    }

    // ─── AVX 指令（需要 VEX 前缀）───

    /// VEX.128.66.0F.WIG FC /r  VPSUBB — 字节整数向量减法 (AVX)
    pub fn vpsubb_128(&mut self, dst: SimdReg, src1: SimdReg, src2: SimdReg) {
        let vex = 0xC5u8;
        let r_b = !(src1.idx() & 7) & 0xF;
        let vex2 = 0x79 | (r_b << 3);  // L=0(128b), pp=01(66)
        self.bytes.extend_from_slice(&[vex, vex2, 0xFC,
            Self::modrm_rr(dst.idx(), src2.idx())]);
    }

    // ─── 向量化循环代码生成模板 ───

    /// 生成向量化循环的序言代码
    /// 假设：rsi=源数组, rdi=目标数组, rcx=元素数
    /// 每次处理 4 个 i32 元素
    pub fn emit_vectorized_loop_prologue(&mut self) -> Vec<u8> {
        let saved = self.bytes.clone();
        self.bytes.clear();
        // 零化累加向量: xmm0 = 0
        self.zero_xmm(SimdReg::Xmm(0));
        // 循环计数器对齐检查（保留rcx % 4 的标量尾巴处理）
        // mov eax, ecx; and eax, 3  → 尾巴元素数
        self.bytes.extend_from_slice(&[0x89, 0xC8, 0x83, 0xE0, 0x03]);
        let result = self.bytes.clone();
        self.bytes = saved;
        result
    }

    /// 生成向量化循环的核心体（4×i32加法）
    pub fn emit_vec4_i32_add(&mut self) -> Vec<u8> {
        let saved = self.bytes.clone();
        self.bytes.clear();
        // MOVDQU xmm1, [rsi]  — 加载4个i32
        self.emit3(0xF3, 0x0F, 0x6F);
        self.emit2(0x0E, 0x06);  // movdqu xmm1, [rsi]
        // PADDD xmm0, xmm1
        self.paddd(SimdReg::Xmm(0), SimdReg::Xmm(1));
        // add rsi, 16; sub rcx, 4
        self.bytes.extend_from_slice(&[0x48, 0x83, 0xC6, 0x10]);  // add rsi, 16
        self.bytes.extend_from_slice(&[0x48, 0x83, 0xE9, 0x04]);  // sub rcx, 4
        let result = self.bytes.clone();
        self.bytes = saved;
        result
    }

    /// 水平加法：xmm0[0] + xmm0[1] + xmm0[2] + xmm0[3]
    /// 使用 PHADDD (SSSE3) 或手动 PSHUFD+PADDD
    pub fn emit_horizontal_sum_i32(&mut self) -> Vec<u8> {
        let saved = self.bytes.clone();
        self.bytes.clear();
        // PSHUFD xmm1, xmm0, 0x4E  → xmm1 = [xmm0[2], xmm0[3], xmm0[0], xmm0[1]]
        self.pshufd(SimdReg::Xmm(1), SimdReg::Xmm(0), 0x4E);
        // PADDD xmm0, xmm1
        self.paddd(SimdReg::Xmm(0), SimdReg::Xmm(1));
        // PSHUFD xmm1, xmm0, 0xB1
        self.pshufd(SimdReg::Xmm(1), SimdReg::Xmm(0), 0xB1);
        // PADDD xmm0, xmm1  → xmm0[0] 现在包含所有4个元素的和
        self.paddd(SimdReg::Xmm(0), SimdReg::Xmm(1));
        // MOVD eax, xmm0  — 提取结果到 eax
        self.emit4(0x66, 0x0F, 0x7E, 0xC0);
        let result = self.bytes.clone();
        self.bytes = saved;
        result
    }
}

impl Default for SimdEmitter {
    fn default() -> Self { Self::new() }
}

/// 检测当前 CPU 支持的 SIMD 特性（x86-64 CPUID）
#[cfg(target_arch = "x86_64")]
pub fn detect_simd_features() -> SimdFeatures {
    use std::arch::x86_64::__cpuid;
    // Rust 1.59+ 已标记 __cpuid 为 safe intrinsic, 移除多余unsafe块 (2026-04-17 A.4)
    let leaf1 = __cpuid(1);
    let leaf7 = __cpuid(7);
    SimdFeatures {
        sse2:    leaf1.edx & (1 << 26) != 0,
        ssse3:   leaf1.ecx & (1 << 9)  != 0,
        sse4_1:  leaf1.ecx & (1 << 19) != 0,
        avx:     leaf1.ecx & (1 << 28) != 0,
        avx2:    leaf7.ebx & (1 << 5)  != 0,
        avx512f: leaf7.ebx & (1 << 16) != 0,
        bmi2:    leaf7.ebx & (1 << 8)  != 0,
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn detect_simd_features() -> SimdFeatures {
    SimdFeatures::default()
}

#[derive(Debug, Default, Clone)]
pub struct SimdFeatures {
    pub sse2:    bool,
    pub ssse3:   bool,
    pub sse4_1:  bool,
    pub avx:     bool,
    pub avx2:    bool,
    pub avx512f: bool,
    pub bmi2:    bool,
}

impl SimdFeatures {
    pub fn best_width(&self) -> usize {
        if self.avx512f { 512 }
        else if self.avx2 { 256 }
        else if self.sse2 { 128 }
        else { 64 }
    }
    pub fn format(&self) -> String {
        let mut feats = vec!["SSE2"];
        if self.ssse3 { feats.push("SSSE3"); }
        if self.sse4_1 { feats.push("SSE4.1"); }
        if self.avx { feats.push("AVX"); }
        if self.avx2 { feats.push("AVX2"); }
        if self.avx512f { feats.push("AVX-512F"); }
        if self.bmi2 { feats.push("BMI2"); }
        feats.join("+")
    }
}
