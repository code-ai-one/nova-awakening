#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 调试信息生成助手 (Debug Info / DWARF)
/// 生成 DWARF v4 格式的调试信息，支持源码级调试
/// 覆盖：编译单元/函数/变量/行号表/类型信息

use std::collections::HashMap;

/// DWARF 标签（简化子集）
#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum DwTag {
    CompileUnit        = 0x11,
    Subprogram         = 0x2E,
    Variable           = 0x34,
    FormalParameter    = 0x05,
    BaseType           = 0x24,
    PointerType        = 0x0F,
    ArrayType          = 0x01,
    StructureType      = 0x13,
    LexicalBlock       = 0x0B,
    Inlined            = 0x1D,
}

/// DWARF 属性编码（简化子集）
#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum DwAt {
    Name       = 0x03,
    Language   = 0x13,
    CompDir    = 0x1B,
    LowPc      = 0x11,
    HighPc     = 0x12,
    ByteSize   = 0x0B,
    Encoding   = 0x3E,
    External   = 0x3F,
    DeclFile   = 0x3A,
    DeclLine   = 0x3B,
    Type       = 0x49,
    Location   = 0x02,
    FrameBase  = 0x40,
}

/// DWARF 属性值
#[derive(Debug, Clone)]
pub enum DwAtVal {
    Str(String),
    U64(u64),
    I64(i64),
    Bool(bool),
    Ref(u32),     // 指向另一个 DIE 的引用
    Bytes(Vec<u8>),  // 表达式/位置
}

/// Debug Information Entry (DIE)
#[derive(Debug, Clone)]
pub struct Die {
    pub id:       u32,
    pub tag:      DwTag,
    pub attrs:    HashMap<u16, DwAtVal>,
    pub children: Vec<u32>,
}

impl Die {
    pub fn new(id: u32, tag: DwTag) -> Self {
        Die { id, tag, attrs: HashMap::new(), children: vec![] }
    }
    pub fn set(&mut self, attr: DwAt, val: DwAtVal) -> &mut Self {
        self.attrs.insert(attr as u16, val);
        self
    }
    pub fn set_name(&mut self, name: &str) -> &mut Self { self.set(DwAt::Name, DwAtVal::Str(name.to_string())) }
    pub fn set_low_pc(&mut self, pc: u64) -> &mut Self { self.set(DwAt::LowPc, DwAtVal::U64(pc)) }
    pub fn set_high_pc(&mut self, pc: u64) -> &mut Self { self.set(DwAt::HighPc, DwAtVal::U64(pc)) }
    pub fn set_type(&mut self, type_ref: u32) -> &mut Self { self.set(DwAt::Type, DwAtVal::Ref(type_ref)) }
}

/// 行号表条目
#[derive(Debug, Clone)]
pub struct LineEntry {
    pub pc:      u64,   // 程序计数器
    pub file:    u32,   // 文件编号
    pub line:    u32,   // 源码行号
    pub col:     u16,   // 列号（0=未知）
    pub is_stmt: bool,  // 是否是语句的开始
    pub end_seq: bool,  // 是否是序列结束
}

/// 调试信息容器
pub struct DebugInfoBuilder {
    dies:         Vec<Die>,
    next_die_id:  u32,
    line_table:   Vec<LineEntry>,
    files:        Vec<String>,  // 文件名表（index+1 为 DWARF 文件编号）
    types:        HashMap<String, u32>,  // 类型名 → DIE ID
}

impl DebugInfoBuilder {
    pub fn new() -> Self {
        let mut builder = DebugInfoBuilder {
            dies: vec![], next_die_id: 1, line_table: vec![], files: vec![], types: HashMap::new()
        };
        builder.create_base_types();
        builder
    }

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_die_id;
        self.next_die_id += 1;
        id
    }

    fn create_base_types(&mut self) {
        for (name, size, enc) in &[
            ("整数", 8u8, 5u8),  // DW_ATE_signed
            ("浮点", 8u8, 4u8),  // DW_ATE_float
            ("布尔", 1u8, 2u8),  // DW_ATE_boolean
            ("字节", 1u8, 8u8),  // DW_ATE_unsigned_char
        ] {
            let id = self.alloc_id();
            let mut die = Die::new(id, DwTag::BaseType);
            die.set_name(name);
            die.set(DwAt::ByteSize, DwAtVal::U64(*size as u64));
            die.set(DwAt::Encoding, DwAtVal::U64(*enc as u64));
            self.dies.push(die);
            self.types.insert(name.to_string(), id);
        }
    }

    /// 注册源文件
    pub fn add_file(&mut self, path: &str) -> u32 {
        if let Some(i) = self.files.iter().position(|f| f == path) {
            return (i + 1) as u32;
        }
        self.files.push(path.to_string());
        self.files.len() as u32
    }

    /// 创建编译单元 DIE
    pub fn create_compile_unit(&mut self, source_file: &str, comp_dir: &str) -> u32 {
        let file_idx = self.add_file(source_file);
        let id = self.alloc_id();
        let mut die = Die::new(id, DwTag::CompileUnit);
        die.set_name(source_file);
        die.set(DwAt::Language,  DwAtVal::U64(0x1D));  // DW_LANG_C99（占位）
        die.set(DwAt::CompDir,   DwAtVal::Str(comp_dir.to_string()));
        self.dies.push(die);
        id
    }

    /// 创建函数 DIE
    pub fn create_function(&mut self, name: &str, low_pc: u64, high_pc: u64, file: u32, line: u32) -> u32 {
        let id = self.alloc_id();
        let mut die = Die::new(id, DwTag::Subprogram);
        die.set_name(name)
            .set_low_pc(low_pc)
            .set_high_pc(high_pc)
            .set(DwAt::DeclFile, DwAtVal::U64(file as u64))
            .set(DwAt::DeclLine, DwAtVal::U64(line as u64))
            .set(DwAt::External,  DwAtVal::Bool(true));
        // 帧基址：x86-64 使用 rbp
        die.set(DwAt::FrameBase, DwAtVal::Bytes(vec![0x86]));  // DW_OP_fbreg
        self.dies.push(die);
        id
    }

    /// 创建局部变量 DIE
    pub fn create_variable(&mut self, name: &str, type_name: &str, fp_offset: i32) -> u32 {
        let type_ref = self.types.get(type_name).copied().unwrap_or(1);
        let id = self.alloc_id();
        let mut die = Die::new(id, DwTag::Variable);
        die.set_name(name).set_type(type_ref);
        // 位置表达式：DW_OP_fbreg + offset
        let mut loc = vec![0x77u8];  // DW_OP_fbreg
        // 编码 SLEB128 offset
        let mut val = fp_offset;
        loop {
            let byte = (val & 0x7F) as u8;
            val >>= 7;
            let done = (val == 0 && byte & 0x40 == 0) || (val == -1 && byte & 0x40 != 0);
            loc.push(if !done { byte | 0x80 } else { byte });
            if done { break; }
        }
        die.set(DwAt::Location, DwAtVal::Bytes(loc));
        self.dies.push(die);
        id
    }

    /// 添加行号表条目
    pub fn add_line(&mut self, pc: u64, file: u32, line: u32, col: u16) {
        self.line_table.push(LineEntry { pc, file, line, col, is_stmt: true, end_seq: false });
    }

    /// 结束行号序列
    pub fn end_sequence(&mut self, pc: u64) {
        if let Some(last) = self.line_table.last().map(|e| e.file) {
            self.line_table.push(LineEntry { pc, file: last, line: 0, col: 0, is_stmt: false, end_seq: true });
        }
    }

    /// 获取函数的 PC 范围（用于调试）
    pub fn pc_range_for_func(&self, func_die_id: u32) -> Option<(u64, u64)> {
        let die = self.dies.iter().find(|d| d.id == func_die_id)?;
        let low = if let Some(DwAtVal::U64(v)) = die.attrs.get(&(DwAt::LowPc as u16)) { *v } else { return None; };
        let high = if let Some(DwAtVal::U64(v)) = die.attrs.get(&(DwAt::HighPc as u16)) { *v } else { return None; };
        Some((low, high))
    }

    /// 统计
    pub fn stats(&self) -> DebugStats {
        DebugStats {
            dies:        self.dies.len(),
            line_entries: self.line_table.len(),
            files:       self.files.len(),
            types:       self.types.len(),
        }
    }
}

impl Default for DebugInfoBuilder { fn default() -> Self { Self::new() } }

#[derive(Debug)]
pub struct DebugStats {
    pub dies:         usize,
    pub line_entries: usize,
    pub files:        usize,
    pub types:        usize,
}
impl DebugStats {
    pub fn format(&self) -> String {
        format!("调试信息: {}个DIE {}条行号 {}个文件 {}个类型",
            self.dies, self.line_entries, self.files, self.types)
    }
}
