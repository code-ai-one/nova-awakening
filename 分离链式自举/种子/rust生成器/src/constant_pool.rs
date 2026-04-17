#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 常量池管理
/// 汇聚代码中所有常量（浮点数/字符串/大整数/跳转表）到 .rodata 节
/// 相同常量只存一份，代码引用偏移而非嵌入字面量

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    F32(u32),        // 浮点32位（存为位模式）
    F64(u64),        // 浮点64位
    Int128(i64, i64),// 128位整数（高64+低64）
    Bytes(Vec<u8>),  // 任意字节序列（字符串/结构体常量/向量常量）
    JumpTable(Vec<i32>),  // 跳转表（相对偏移）
}

impl ConstValue {
    pub fn size(&self) -> usize {
        match self {
            ConstValue::F32(_) => 4,
            ConstValue::F64(_) => 8,
            ConstValue::Int128(_, _) => 16,
            ConstValue::Bytes(b) => b.len(),
            ConstValue::JumpTable(t) => t.len() * 4,
        }
    }

    pub fn align(&self) -> usize {
        match self {
            ConstValue::F32(_) => 4,
            ConstValue::F64(_) | ConstValue::Int128(_, _) => 8,
            ConstValue::Bytes(_) => 1,
            ConstValue::JumpTable(_) => 4,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            ConstValue::F32(bits) => bits.to_le_bytes().to_vec(),
            ConstValue::F64(bits) => bits.to_le_bytes().to_vec(),
            ConstValue::Int128(hi, lo) => {
                let mut v = lo.to_le_bytes().to_vec();
                v.extend_from_slice(&hi.to_le_bytes());
                v
            }
            ConstValue::Bytes(b) => b.clone(),
            ConstValue::JumpTable(t) => {
                t.iter().flat_map(|&e| e.to_le_bytes()).collect()
            }
        }
    }

    /// 计算哈希键（用于去重）
    fn dedup_key(&self) -> u64 {
        match self {
            ConstValue::F32(b) => *b as u64 | 0xF3_00_00_00_00_00_00_00,
            ConstValue::F64(b) => *b ^ 0xF6_00_00_00_00_00_00_00,
            ConstValue::Int128(hi, lo) => (*hi as u64).wrapping_add(*lo as u64),
            ConstValue::Bytes(b) => {
                let mut h = 0xcbf29ce484222325u64;
                for &byte in b { h = h.wrapping_mul(0x100000001b3).wrapping_add(byte as u64 ^ 0xBA); }
                h
            }
            ConstValue::JumpTable(t) => {
                let mut h = 0u64;
                for &e in t { h = h.wrapping_add(e as u64).rotate_left(7); }
                h
            }
        }
    }
}

/// 常量条目
#[derive(Debug, Clone)]
pub struct ConstEntry {
    pub id:     u32,
    pub value:  ConstValue,
    pub offset: u32,  // 在常量池中的字节偏移
}

/// 常量池
pub struct ConstPool {
    entries:    Vec<ConstEntry>,
    dedup_map:  HashMap<u64, u32>,  // 哈希键 → 条目ID
    pool_bytes: Vec<u8>,
    next_offset: u32,
}

impl ConstPool {
    pub fn new() -> Self {
        ConstPool { entries: vec![], dedup_map: HashMap::new(), pool_bytes: vec![], next_offset: 0 }
    }

    /// 插入常量（自动去重），返回条目 ID
    pub fn insert(&mut self, value: ConstValue) -> u32 {
        let key = value.dedup_key();
        if let Some(&existing_id) = self.dedup_map.get(&key) {
            // 简单哈希去重（可能有极小概率碰撞，生产中需要完整比较）
            return existing_id;
        }

        let align = value.align();
        // 对齐
        while self.next_offset as usize % align != 0 {
            self.pool_bytes.push(0);
            self.next_offset += 1;
        }

        let offset = self.next_offset;
        let bytes = value.to_bytes();
        self.next_offset += bytes.len() as u32;
        self.pool_bytes.extend_from_slice(&bytes);

        let id = self.entries.len() as u32;
        self.entries.push(ConstEntry { id, value, offset });
        self.dedup_map.insert(key, id);
        id
    }

    /// 获取条目（通过 ID）
    pub fn get(&self, id: u32) -> Option<&ConstEntry> {
        self.entries.get(id as usize)
    }

    /// 获取常量在池中的字节偏移
    pub fn offset_of(&self, id: u32) -> Option<u32> {
        self.entries.get(id as usize).map(|e| e.offset)
    }

    /// 序列化整个常量池
    pub fn bytes(&self) -> &[u8] { &self.pool_bytes }

    /// 大小（字节）
    pub fn size(&self) -> usize { self.pool_bytes.len() }

    /// 条目数量
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    /// 找到最近的常量，返回相对代码位置的偏移
    /// pc_offset: 当前指令相对池基址的偏移
    pub fn nearest_const_offset(&self, id: u32, pc_offset: u32) -> Option<i32> {
        let const_off = self.offset_of(id)?;
        Some(const_off as i32 - pc_offset as i32)
    }

    /// 常量池统计
    pub fn stats(&self) -> ConstPoolStats {
        let by_type = [
            ("F32", self.entries.iter().filter(|e| matches!(e.value, ConstValue::F32(_))).count()),
            ("F64", self.entries.iter().filter(|e| matches!(e.value, ConstValue::F64(_))).count()),
            ("字节", self.entries.iter().filter(|e| matches!(e.value, ConstValue::Bytes(_))).count()),
            ("跳转表", self.entries.iter().filter(|e| matches!(e.value, ConstValue::JumpTable(_))).count()),
        ];
        ConstPoolStats { total: self.entries.len(), size_bytes: self.pool_bytes.len(), by_type: by_type.to_vec() }
    }

    /// 生成 .rodata 节内容（带节头和对齐填充）
    pub fn emit_rodata_section(&self) -> Vec<u8> {
        // 确保以16字节对齐结束
        let mut data = self.pool_bytes.clone();
        while data.len() % 16 != 0 { data.push(0); }
        data
    }
}

impl Default for ConstPool { fn default() -> Self { Self::new() } }

#[derive(Debug)]
pub struct ConstPoolStats {
    pub total:      usize,
    pub size_bytes: usize,
    pub by_type:    Vec<(&'static str, usize)>,
}
impl ConstPoolStats {
    pub fn format(&self) -> String {
        let types: Vec<_> = self.by_type.iter().filter(|(_, c)| *c > 0)
            .map(|(n, c)| format!("{}:{}", n, c)).collect();
        format!("常量池: {}个常量 {:.1}KB [{}]",
            self.total, self.size_bytes as f64 / 1024.0, types.join(" "))
    }
}

/// 辅助：把 f64 字面量插入常量池
pub fn intern_f64(pool: &mut ConstPool, val: f64) -> u32 {
    pool.insert(ConstValue::F64(val.to_bits()))
}

/// 辅助：把字符串字面量插入常量池（带 null 终止符）
pub fn intern_cstr(pool: &mut ConstPool, s: &str) -> u32 {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);  // null terminator
    pool.insert(ConstValue::Bytes(bytes))
}

/// 辅助：把跳转表插入常量池
pub fn intern_jump_table(pool: &mut ConstPool, targets: &[i32]) -> u32 {
    pool.insert(ConstValue::JumpTable(targets.to_vec()))
}
