#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova WebAssembly 后端基础
/// 把 Nova 字节码转换为 WebAssembly 二进制格式（MVP）
/// 支持：整数/浮点运算、控制流、局部变量、内存操作

/// WASM 值类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmType { I32, I64, F32, F64 }
impl WasmType {
    pub fn byte(self) -> u8 {
        match self { WasmType::I32 => 0x7F, WasmType::I64 => 0x7E, WasmType::F32 => 0x7D, WasmType::F64 => 0x7C }
    }
}

/// WASM 操作码（MVP子集）
#[derive(Debug, Clone, Copy)]
pub enum WasmOp {
    Unreachable = 0x00, Nop = 0x01,
    Block = 0x02, Loop = 0x03, If = 0x04, Else = 0x05, End = 0x0B,
    Br = 0x0C, BrIf = 0x0D, Return = 0x0F,
    Call = 0x10, CallIndirect = 0x11,
    Drop = 0x1A, Select = 0x1B,
    LocalGet = 0x20, LocalSet = 0x21, LocalTee = 0x22,
    GlobalGet = 0x23, GlobalSet = 0x24,
    I32Load = 0x28, I64Load = 0x29, I32Store = 0x36, I64Store = 0x37,
    MemorySize = 0x3F, MemoryGrow = 0x40,
    I32Const = 0x41, I64Const = 0x42, F32Const = 0x43, F64Const = 0x44,
    I32Eqz = 0x45, I32Eq = 0x46, I32Ne = 0x47,
    I32LtS = 0x48, I32GtS = 0x4A, I32LeS = 0x4C, I32GeS = 0x4E,
    I64Eqz = 0x50, I64Eq = 0x51, I64Ne = 0x52,
    I32Add = 0x6A, I32Sub = 0x6B, I32Mul = 0x6C,
    I32DivS = 0x6D, I32RemS = 0x6F, I32And = 0x71, I32Or = 0x72, I32Xor = 0x73,
    I32Shl = 0x74, I32ShrS = 0x75,
    I64Add = 0x7C, I64Sub = 0x7D, I64Mul = 0x7E, I64DivS = 0x7F,
    F64Add = 0xA0, F64Sub = 0xA1, F64Mul = 0xA2, F64Div = 0xA3,
    I32WrapI64 = 0xA7, I64ExtendI32S = 0xAC,
    F64ConvertI64S = 0xB9,
}

/// WASM 函数签名
#[derive(Debug, Clone)]
pub struct WasmFuncType {
    pub params:  Vec<WasmType>,
    pub results: Vec<WasmType>,
}

/// WASM 函数
#[derive(Debug, Clone)]
pub struct WasmFunc {
    pub type_idx: u32,
    pub locals:   Vec<(u32, WasmType)>,  // (count, type)
    pub body:     Vec<u8>,
}

/// WASM 模块构建器
pub struct WasmModule {
    pub types:    Vec<WasmFuncType>,
    pub imports:  Vec<WasmImport>,
    pub funcs:    Vec<WasmFunc>,
    pub exports:  Vec<WasmExport>,
    pub memory:   Option<(u32, Option<u32>)>,  // (min_pages, max_pages)
    pub data:     Vec<WasmDataSegment>,
}

#[derive(Debug, Clone)]
pub struct WasmImport {
    pub module: String,
    pub name:   String,
    pub kind:   WasmImportKind,
}
#[derive(Debug, Clone)]
pub enum WasmImportKind { Func(u32), Memory(u32, Option<u32>) }

#[derive(Debug, Clone)]
pub struct WasmExport {
    pub name:  String,
    pub kind:  u8,   // 0=func, 1=table, 2=mem, 3=global
    pub index: u32,
}

#[derive(Debug, Clone)]
pub struct WasmDataSegment {
    pub offset: u32,
    pub data:   Vec<u8>,
}

impl WasmModule {
    pub fn new() -> Self {
        WasmModule { types: vec![], imports: vec![], funcs: vec![], exports: vec![], memory: None, data: vec![] }
    }

    pub fn add_type(&mut self, ft: WasmFuncType) -> u32 {
        // 复用已有类型
        if let Some(i) = self.types.iter().position(|t| t.params == ft.params && t.results == ft.results) {
            return i as u32;
        }
        let i = self.types.len() as u32;
        self.types.push(ft);
        i
    }

    pub fn add_func(&mut self, func: WasmFunc) -> u32 {
        let i = (self.imports.iter().filter(|imp| matches!(imp.kind, WasmImportKind::Func(_))).count() + self.funcs.len()) as u32;
        self.funcs.push(func);
        i
    }

    pub fn export_func(&mut self, name: impl Into<String>, idx: u32) {
        self.exports.push(WasmExport { name: name.into(), kind: 0, index: idx });
    }

    pub fn set_memory(&mut self, min_pages: u32) {
        self.memory = Some((min_pages, None));
    }

    /// 序列化为 WASM 二进制格式
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![0x00, 0x61, 0x73, 0x6D,  // magic: \0asm
                           0x01, 0x00, 0x00, 0x00];  // version: 1

        // Type Section (1)
        if !self.types.is_empty() {
            let mut sec = vec![];
            leb_u32(&mut sec, self.types.len() as u32);
            for ft in &self.types {
                sec.push(0x60);  // func type marker
                leb_u32(&mut sec, ft.params.len() as u32);
                for t in &ft.params { sec.push(t.byte()); }
                leb_u32(&mut sec, ft.results.len() as u32);
                for t in &ft.results { sec.push(t.byte()); }
            }
            out.push(1);
            leb_u32(&mut out, sec.len() as u32);
            out.extend(sec);
        }

        // Import Section (2)
        if !self.imports.is_empty() {
            let mut sec = vec![];
            leb_u32(&mut sec, self.imports.len() as u32);
            for imp in &self.imports {
                let mod_bytes = imp.module.as_bytes();
                leb_u32(&mut sec, mod_bytes.len() as u32);
                sec.extend(mod_bytes);
                let name_bytes = imp.name.as_bytes();
                leb_u32(&mut sec, name_bytes.len() as u32);
                sec.extend(name_bytes);
                match imp.kind {
                    WasmImportKind::Func(t) => { sec.push(0x00); leb_u32(&mut sec, t); }
                    WasmImportKind::Memory(min, max) => {
                        sec.push(0x02);
                        if let Some(m) = max { sec.push(0x01); leb_u32(&mut sec, min); leb_u32(&mut sec, m); }
                        else { sec.push(0x00); leb_u32(&mut sec, min); }
                    }
                }
            }
            out.push(2);
            leb_u32(&mut out, sec.len() as u32);
            out.extend(sec);
        }

        // Function Section (3)
        if !self.funcs.is_empty() {
            let mut sec = vec![];
            leb_u32(&mut sec, self.funcs.len() as u32);
            for f in &self.funcs { leb_u32(&mut sec, f.type_idx); }
            out.push(3);
            leb_u32(&mut out, sec.len() as u32);
            out.extend(sec);
        }

        // Memory Section (5)
        if let Some((min, max)) = self.memory {
            let mut sec = vec![];
            sec.push(1);  // count
            if let Some(m) = max { sec.push(1); leb_u32(&mut sec, min); leb_u32(&mut sec, m); }
            else { sec.push(0); leb_u32(&mut sec, min); }
            out.push(5);
            leb_u32(&mut out, sec.len() as u32);
            out.extend(sec);
        }

        // Export Section (7)
        if !self.exports.is_empty() {
            let mut sec = vec![];
            leb_u32(&mut sec, self.exports.len() as u32);
            for exp in &self.exports {
                let nb = exp.name.as_bytes();
                leb_u32(&mut sec, nb.len() as u32);
                sec.extend(nb);
                sec.push(exp.kind);
                leb_u32(&mut sec, exp.index);
            }
            out.push(7);
            leb_u32(&mut out, sec.len() as u32);
            out.extend(sec);
        }

        // Code Section (10)
        if !self.funcs.is_empty() {
            let mut sec = vec![];
            leb_u32(&mut sec, self.funcs.len() as u32);
            for func in &self.funcs {
                let mut fbody = vec![];
                // locals
                leb_u32(&mut fbody, func.locals.len() as u32);
                for (count, ty) in &func.locals { leb_u32(&mut fbody, *count); fbody.push(ty.byte()); }
                fbody.extend(&func.body);
                fbody.push(0x0B);  // end
                leb_u32(&mut sec, fbody.len() as u32);
                sec.extend(fbody);
            }
            out.push(10);
            leb_u32(&mut out, sec.len() as u32);
            out.extend(sec);
        }

        // Data Section (11)
        if !self.data.is_empty() {
            let mut sec = vec![];
            leb_u32(&mut sec, self.data.len() as u32);
            for seg in &self.data {
                sec.push(0);  // active, memory 0
                sec.push(0x41);  // i32.const
                leb_i32(&mut sec, seg.offset as i32);
                sec.push(0x0B);  // end
                leb_u32(&mut sec, seg.data.len() as u32);
                sec.extend(&seg.data);
            }
            out.push(11);
            leb_u32(&mut out, sec.len() as u32);
            out.extend(sec);
        }

        out
    }

    pub fn write(&self, path: &std::path::Path) -> Result<(), String> {
        std::fs::write(path, self.encode())
            .map_err(|e| format!("写入WASM失败: {}", e))
    }
}

impl Default for WasmModule { fn default() -> Self { Self::new() } }

// LEB128 编码辅助
fn leb_u32(out: &mut Vec<u8>, mut v: u32) {
    loop {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        out.push(if v != 0 { byte | 0x80 } else { byte });
        if v == 0 { break; }
    }
}

fn leb_i32(out: &mut Vec<u8>, mut v: i32) {
    let mut more = true;
    while more {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        more = !((v == 0 && byte & 0x40 == 0) || (v == -1 && byte & 0x40 != 0));
        out.push(if more { byte | 0x80 } else { byte });
    }
}

/// 代码生成辅助函数
pub fn emit_i32_const(body: &mut Vec<u8>, v: i32) {
    body.push(WasmOp::I32Const as u8);
    leb_i32(body, v);
}
pub fn emit_i64_const(body: &mut Vec<u8>, v: i64) {
    body.push(WasmOp::I64Const as u8);
    let mut tmp = vec![];
    leb_u32(&mut tmp, v.unsigned_abs() as u32 + if v < 0 { 0x80 } else { 0 });
    body.extend(tmp);
}
pub fn emit_local_get(body: &mut Vec<u8>, idx: u32) { body.push(0x20); leb_u32(body, idx); }
pub fn emit_local_set(body: &mut Vec<u8>, idx: u32) { body.push(0x21); leb_u32(body, idx); }
pub fn emit_call(body: &mut Vec<u8>, func_idx: u32) { body.push(0x10); leb_u32(body, func_idx); }
pub fn emit_return(body: &mut Vec<u8>) { body.push(WasmOp::Return as u8); }
pub fn emit_end(body: &mut Vec<u8>) { body.push(WasmOp::End as u8); }
pub fn emit_op(body: &mut Vec<u8>, op: WasmOp) { body.push(op as u8); }
