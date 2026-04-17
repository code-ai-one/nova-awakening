#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 字符串驻留 (String Interning)
/// 编译器中字符串比较的核心优化：相同字符串只存一份，比较 O(1)
/// 符号名/类型名/标识符全部通过 Interned ID 比较

use std::collections::HashMap;

/// 驻留字符串的 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InternedStr(u32);

impl InternedStr {
    pub fn idx(self) -> u32 { self.0 }
    pub const EMPTY: InternedStr = InternedStr(0);
}

/// 字符串驻留表
pub struct StringInterner {
    table:    HashMap<String, InternedStr>,
    strings:  Vec<String>,
}

impl StringInterner {
    pub fn new() -> Self {
        let mut si = StringInterner { table: HashMap::new(), strings: vec![] };
        si.intern("");  // ID 0 = 空字符串
        si
    }

    /// 驻留字符串，返回唯一 ID
    pub fn intern(&mut self, s: &str) -> InternedStr {
        if let Some(&id) = self.table.get(s) { return id; }
        let id = InternedStr(self.strings.len() as u32);
        self.strings.push(s.to_string());
        self.table.insert(s.to_string(), id);
        id
    }

    /// 通过 ID 获取字符串
    pub fn get(&self, id: InternedStr) -> &str {
        self.strings.get(id.0 as usize).map(|s| s.as_str()).unwrap_or("")
    }

    /// 检查字符串是否已驻留
    pub fn lookup(&self, s: &str) -> Option<InternedStr> {
        self.table.get(s).copied()
    }

    pub fn len(&self) -> usize { self.strings.len() }
    pub fn is_empty(&self) -> bool { self.strings.len() <= 1 }

    /// 批量驻留（从迭代器）
    pub fn intern_all<'a>(&mut self, strs: impl Iterator<Item = &'a str>) -> Vec<InternedStr> {
        strs.map(|s| self.intern(s)).collect()
    }

    /// 生成统计（用于分析编译器中字符串重复度）
    pub fn stats(&self) -> InternStats {
        let total_bytes: usize = self.strings.iter().map(|s| s.len()).sum();
        let max_len = self.strings.iter().map(|s| s.len()).max().unwrap_or(0);
        InternStats { total_strings: self.strings.len(), total_bytes, max_len,
                      avg_len: if self.strings.is_empty() { 0.0 } else { total_bytes as f64 / self.strings.len() as f64 } }
    }

    /// 序列化（用于缓存持久化）
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = vec![];
        out.extend_from_slice(&(self.strings.len() as u32).to_le_bytes());
        for s in &self.strings {
            let b = s.as_bytes();
            out.extend_from_slice(&(b.len() as u32).to_le_bytes());
            out.extend_from_slice(b);
        }
        out
    }

    /// 反序列化
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 4 { return None; }
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let mut si = StringInterner { table: HashMap::new(), strings: vec![] };
        let mut pos = 4;
        for _ in 0..count {
            if pos + 4 > data.len() { return None; }
            let len = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
            pos += 4;
            if pos + len > data.len() { return None; }
            let s = std::str::from_utf8(&data[pos..pos+len]).ok()?.to_string();
            let id = InternedStr(si.strings.len() as u32);
            si.table.insert(s.clone(), id);
            si.strings.push(s);
            pos += len;
        }
        Some(si)
    }
}

impl Default for StringInterner { fn default() -> Self { Self::new() } }

#[derive(Debug)]
pub struct InternStats {
    pub total_strings: usize,
    pub total_bytes:   usize,
    pub max_len:       usize,
    pub avg_len:       f64,
}
impl InternStats {
    pub fn format(&self) -> String {
        format!("字符串驻留: {}个字符串 {:.1}KB 最长{}字符 均长{:.1}字符",
            self.total_strings, self.total_bytes as f64 / 1024.0, self.max_len, self.avg_len)
    }
}

// 全局字符串驻留表（线程本地，用于单线程编译器）
thread_local! {
    static GLOBAL_INTERNER: std::cell::RefCell<StringInterner> =
        std::cell::RefCell::new(StringInterner::new());
}

pub fn intern(s: &str) -> InternedStr {
    GLOBAL_INTERNER.with(|si| si.borrow_mut().intern(s))
}
pub fn get_str(id: InternedStr) -> String {
    GLOBAL_INTERNER.with(|si| si.borrow().get(id).to_string())
}
pub fn global_stats() -> InternStats {
    GLOBAL_INTERNER.with(|si| si.borrow().stats())
}
