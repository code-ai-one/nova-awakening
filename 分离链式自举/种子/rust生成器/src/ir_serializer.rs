#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova IR 序列化/反序列化
/// 把 Nova-Bytecode 序列化为可读文本格式（类似 LLVM IR 文本格式）
/// 应用：调试/可视化/跨轮次缓存/差异比较

use std::io::Write;

/// Nova 字节码操作码（85条指令的子集，与 IR/字节码/指令集.nova 对齐）
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NovaOp {
    Nop        = 0x00,
    PushI64    = 0x01,
    PushF64    = 0x02,
    PushStr    = 0x03,
    PushNil    = 0x04,
    PushTrue   = 0x05,
    PushFalse  = 0x06,
    Pop        = 0x07,
    Dup        = 0x08,
    Swap       = 0x09,
    Add        = 0x10,
    Sub        = 0x11,
    Mul        = 0x12,
    Div        = 0x13,
    Mod        = 0x14,
    Neg        = 0x15,
    Not        = 0x16,
    BitNot     = 0x17,
    BitAnd     = 0x18,
    BitOr      = 0x19,
    BitXor     = 0x1A,
    Shl        = 0x1B,
    Shr        = 0x1C,
    LoadLocal  = 0x20,
    StoreLocal = 0x21,
    LoadGlobal = 0x22,
    StoreGlobal= 0x23,
    NewList    = 0x26,
    NewDict    = 0x27,
    GetIndex   = 0x29,
    SetIndex   = 0x2A,
    Jmp        = 0x30,
    JmpIf      = 0x31,
    JmpIfNot   = 0x32,
    Loop       = 0x33,
    CmpEq      = 0x34,
    CmpNe      = 0x35,
    CmpLt      = 0x36,
    CmpLe      = 0x37,
    CmpGt      = 0x38,
    CmpGe      = 0x39,
    Call       = 0x40,
    CallInd    = 0x41,
    CallNative = 0x42,
    Return     = 0x44,
    ReturnNil  = 0x45,
    Closure    = 0x46,
    TailCall   = 0x47,
    Yield      = 0x48,
    TypeCheck  = 0x50,
    IsInt      = 0x52,
    IsFloat    = 0x53,
    IsString   = 0x54,
    IsList     = 0x55,
    IsDict     = 0x56,
    ToString   = 0x60,
    ToInt      = 0x61,
    ToFloat    = 0x62,
    StrLen     = 0x63,
    StrConcat  = 0x64,
    ListLen    = 0x65,
    ListAppend = 0x66,
    ListGet    = 0x67,
    ListSet    = 0x68,
    DictGet    = 0x70,
    DictSet    = 0x71,
    DictHas    = 0x72,
    DictKeys   = 0x73,
    Unknown    = 0xFF,
}

impl NovaOp {
    pub fn from_byte(b: u8) -> Self {
        match b {
            0x00 => NovaOp::Nop,
            0x01 => NovaOp::PushI64,
            0x02 => NovaOp::PushF64,
            0x03 => NovaOp::PushStr,
            0x04 => NovaOp::PushNil,
            0x05 => NovaOp::PushTrue,
            0x06 => NovaOp::PushFalse,
            0x07 => NovaOp::Pop,
            0x08 => NovaOp::Dup,
            0x09 => NovaOp::Swap,
            0x10 => NovaOp::Add,
            0x11 => NovaOp::Sub,
            0x12 => NovaOp::Mul,
            0x13 => NovaOp::Div,
            0x14 => NovaOp::Mod,
            0x15 => NovaOp::Neg,
            0x16 => NovaOp::Not,
            0x17 => NovaOp::BitNot,
            0x18 => NovaOp::BitAnd,
            0x19 => NovaOp::BitOr,
            0x1A => NovaOp::BitXor,
            0x1B => NovaOp::Shl,
            0x1C => NovaOp::Shr,
            0x20 => NovaOp::LoadLocal,
            0x21 => NovaOp::StoreLocal,
            0x22 => NovaOp::LoadGlobal,
            0x23 => NovaOp::StoreGlobal,
            0x26 => NovaOp::NewList,
            0x27 => NovaOp::NewDict,
            0x29 => NovaOp::GetIndex,
            0x2A => NovaOp::SetIndex,
            0x30 => NovaOp::Jmp,
            0x31 => NovaOp::JmpIf,
            0x32 => NovaOp::JmpIfNot,
            0x33 => NovaOp::Loop,
            0x34 => NovaOp::CmpEq,
            0x35 => NovaOp::CmpNe,
            0x36 => NovaOp::CmpLt,
            0x37 => NovaOp::CmpLe,
            0x38 => NovaOp::CmpGt,
            0x39 => NovaOp::CmpGe,
            0x40 => NovaOp::Call,
            0x41 => NovaOp::CallInd,
            0x42 => NovaOp::CallNative,
            0x44 => NovaOp::Return,
            0x45 => NovaOp::ReturnNil,
            0x46 => NovaOp::Closure,
            0x47 => NovaOp::TailCall,
            0x48 => NovaOp::Yield,
            0x50 => NovaOp::TypeCheck,
            0x52 => NovaOp::IsInt,
            0x53 => NovaOp::IsFloat,
            0x54 => NovaOp::IsString,
            0x55 => NovaOp::IsList,
            0x56 => NovaOp::IsDict,
            0x60 => NovaOp::ToString,
            0x61 => NovaOp::ToInt,
            0x62 => NovaOp::ToFloat,
            0x63 => NovaOp::StrLen,
            0x64 => NovaOp::StrConcat,
            0x65 => NovaOp::ListLen,
            0x66 => NovaOp::ListAppend,
            0x67 => NovaOp::ListGet,
            0x68 => NovaOp::ListSet,
            0x70 => NovaOp::DictGet,
            0x71 => NovaOp::DictSet,
            0x72 => NovaOp::DictHas,
            0x73 => NovaOp::DictKeys,
            _ => NovaOp::Unknown,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            NovaOp::Nop => "nop",     NovaOp::PushI64 => "push_i64",
            NovaOp::PushF64 => "push_f64", NovaOp::PushStr => "push_str",
            NovaOp::PushNil => "push_nil", NovaOp::PushTrue => "push_true",
            NovaOp::PushFalse => "push_false",
            NovaOp::Pop => "pop",     NovaOp::Dup => "dup",  NovaOp::Swap => "swap",
            NovaOp::Add => "add",     NovaOp::Sub => "sub",  NovaOp::Mul => "mul",
            NovaOp::Div => "div",     NovaOp::Mod => "mod",  NovaOp::Neg => "neg",
            NovaOp::Not => "not",     NovaOp::BitNot => "bitnot",
            NovaOp::BitAnd => "bitand", NovaOp::BitOr => "bitor",
            NovaOp::BitXor => "bitxor", NovaOp::Shl => "shl", NovaOp::Shr => "shr",
            NovaOp::LoadLocal => "load_local", NovaOp::StoreLocal => "store_local",
            NovaOp::LoadGlobal => "load_global", NovaOp::StoreGlobal => "store_global",
            NovaOp::NewList => "new_list", NovaOp::NewDict => "new_dict",
            NovaOp::GetIndex => "get_index", NovaOp::SetIndex => "set_index",
            NovaOp::Jmp => "jmp", NovaOp::JmpIf => "jmpif", NovaOp::JmpIfNot => "jmpifnot",
            NovaOp::Loop => "loop",
            NovaOp::CmpEq => "cmpeq", NovaOp::CmpNe => "cmpne",
            NovaOp::CmpLt => "cmplt", NovaOp::CmpLe => "cmple",
            NovaOp::CmpGt => "cmpgt", NovaOp::CmpGe => "cmpge",
            NovaOp::Call => "call", NovaOp::CallInd => "call_ind",
            NovaOp::CallNative => "call_native",
            NovaOp::Return => "return", NovaOp::ReturnNil => "return_nil",
            NovaOp::Closure => "closure", NovaOp::TailCall => "tailcall",
            NovaOp::Yield => "yield",
            NovaOp::TypeCheck => "typecheck",
            NovaOp::IsInt => "is_int", NovaOp::IsFloat => "is_float",
            NovaOp::IsString => "is_string", NovaOp::IsList => "is_list",
            NovaOp::IsDict => "is_dict",
            NovaOp::ToString => "tostring", NovaOp::ToInt => "toint",
            NovaOp::ToFloat => "tofloat", NovaOp::StrLen => "strlen",
            NovaOp::StrConcat => "strconcat",
            NovaOp::ListLen => "list_len", NovaOp::ListAppend => "list_append",
            NovaOp::ListGet => "list_get", NovaOp::ListSet => "list_set",
            NovaOp::DictGet => "dict_get", NovaOp::DictSet => "dict_set",
            NovaOp::DictHas => "dict_has", NovaOp::DictKeys => "dict_keys",
            NovaOp::Unknown => "unknown",
        }
    }

    /// 操作数字节数（后跟的数据大小）
    pub fn operand_bytes(self) -> usize {
        match self {
            NovaOp::PushI64 | NovaOp::PushF64 | NovaOp::PushStr => 8,
            NovaOp::LoadLocal | NovaOp::StoreLocal |
            NovaOp::LoadGlobal | NovaOp::StoreGlobal => 2,
            NovaOp::Jmp | NovaOp::JmpIf | NovaOp::JmpIfNot | NovaOp::Loop => 4,
            NovaOp::Call | NovaOp::TailCall => 3,  // 函数ID(2) + 参数数(1)
            NovaOp::NewList | NovaOp::NewDict => 2,
            _ => 0,
        }
    }
}

/// 序列化的字节码指令
#[derive(Debug, Clone)]
pub struct IrInsn {
    pub offset: usize,
    pub op:     NovaOp,
    pub args:   Vec<u8>,
}

/// Nova-Bytecode 序列化器
pub struct IrSerializer;

impl IrSerializer {
    /// 把字节码序列序列化为可读文本
    pub fn serialize_to_text(bytecode: &[u8]) -> String {
        let insns = Self::parse_bytecode(bytecode);
        let mut out = String::new();
        for insn in &insns {
            let args_str = if insn.args.is_empty() {
                String::new()
            } else {
                let hex: String = insn.args.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
                // 尝试解析为整数值
                let val = match insn.args.len() {
                    1 => format!(" ; {}", insn.args[0]),
                    2 => format!(" ; {}", u16::from_le_bytes([insn.args[0], insn.args[1]])),
                    4 => format!(" ; {}", i32::from_le_bytes([insn.args[0], insn.args[1], insn.args[2], insn.args[3]])),
                    8 => format!(" ; {}", i64::from_le_bytes(insn.args[0..8].try_into().unwrap_or([0;8]))),
                    _ => String::new(),
                };
                format!("  {}{}",hex, val)
            };
            out += &format!("{:6x}:  {:20}{}\n", insn.offset, insn.op.name(), args_str);
        }
        out
    }

    /// 把字节码写入二进制文件（直接复制）
    pub fn write_binary(bytecode: &[u8], path: &std::path::Path) -> Result<(), String> {
        std::fs::write(path, bytecode)
            .map_err(|e| format!("写入字节码失败: {}", e))
    }

    /// 解析字节码序列
    pub fn parse_bytecode(bytecode: &[u8]) -> Vec<IrInsn> {
        let mut result = vec![];
        let mut i = 0;
        while i < bytecode.len() {
            let offset = i;
            let op = NovaOp::from_byte(bytecode[i]);
            i += 1;
            let operand_bytes = op.operand_bytes();
            let args: Vec<u8> = if i + operand_bytes <= bytecode.len() {
                let a = bytecode[i..i+operand_bytes].to_vec();
                i += operand_bytes;
                a
            } else {
                i = bytecode.len();
                vec![]
            };
            result.push(IrInsn { offset, op, args });
        }
        result
    }

    /// 统计指令分布
    pub fn analyze_distribution(bytecode: &[u8]) -> Vec<(String, usize)> {
        let insns = Self::parse_bytecode(bytecode);
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for insn in &insns {
            *counts.entry(insn.op.name().to_string()).or_default() += 1;
        }
        let mut result: Vec<(String, usize)> = counts.into_iter().collect();
        result.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
        result
    }

    /// 比较两个字节码文件的差异
    pub fn diff_bytecodes(a: &[u8], b: &[u8]) -> BytecodeDiff {
        let a_insns = Self::parse_bytecode(a);
        let b_insns = Self::parse_bytecode(b);
        BytecodeDiff {
            size_a: a.len(),
            size_b: b.len(),
            insn_count_a: a_insns.len(),
            insn_count_b: b_insns.len(),
            identical: a == b,
            first_diff_offset: a.iter().zip(b.iter()).position(|(x, y)| x != y),
        }
    }
}

#[derive(Debug)]
pub struct BytecodeDiff {
    pub size_a:           usize,
    pub size_b:           usize,
    pub insn_count_a:     usize,
    pub insn_count_b:     usize,
    pub identical:        bool,
    pub first_diff_offset: Option<usize>,
}

impl BytecodeDiff {
    pub fn format(&self) -> String {
        if self.identical {
            format!("✓ 相同 ({} 字节, {} 条指令)", self.size_a, self.insn_count_a)
        } else {
            format!("✗ 不同: A={}/{}条 B={}/{}条 首差偏移={:?}",
                self.size_a, self.insn_count_a,
                self.size_b, self.insn_count_b,
                self.first_diff_offset)
        }
    }
}
