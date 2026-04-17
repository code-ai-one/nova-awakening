#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 符号表管理
/// 管理全局/局部/模块符号，支持 ELF 符号导出和链接时查询

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolKind {
    Function,
    Global,
    Constant,
    ExternalRef,  // 未解析的外部符号
    Weak,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolVisibility {
    Public,   // 模块间可见
    Private,  // 模块内部
    Hidden,   // ELF hidden
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name:       String,
    pub kind:       SymbolKind,
    pub visibility: SymbolVisibility,
    pub offset:     u64,    // 在代码段中的偏移
    pub size:       usize,  // 符号大小（字节）
    pub module:     String, // 所属模块
    pub resolved:   bool,   // 是否已解析
}

impl Symbol {
    pub fn function(name: impl Into<String>, offset: u64, size: usize, module: impl Into<String>) -> Self {
        Symbol {
            name: name.into(), kind: SymbolKind::Function,
            visibility: SymbolVisibility::Public,
            offset, size, module: module.into(), resolved: true,
        }
    }
    pub fn global(name: impl Into<String>, offset: u64, module: impl Into<String>) -> Self {
        Symbol {
            name: name.into(), kind: SymbolKind::Global,
            visibility: SymbolVisibility::Public,
            offset, size: 8, module: module.into(), resolved: true,
        }
    }
    pub fn external(name: impl Into<String>) -> Self {
        Symbol {
            name: name.into(), kind: SymbolKind::ExternalRef,
            visibility: SymbolVisibility::Public,
            offset: 0, size: 0, module: "<extern>".into(), resolved: false,
        }
    }
}

/// 符号表
pub struct SymbolTable {
    symbols:  HashMap<String, Symbol>,
    pub resolution_errors: Vec<String>,
}

impl SymbolTable {
    pub fn new() -> Self {
        SymbolTable { symbols: HashMap::new(), resolution_errors: vec![] }
    }

    pub fn define(&mut self, sym: Symbol) -> Result<(), String> {
        let name = sym.name.clone();
        if let Some(existing) = self.symbols.get(&name) {
            if existing.resolved && sym.kind != SymbolKind::Weak {
                return Err(format!("符号重定义: `{}` (在模块 {} 和 {})", name, existing.module, sym.module));
            }
        }
        self.symbols.insert(name, sym);
        Ok(())
    }

    pub fn resolve(&mut self, name: &str, offset: u64, module: &str) -> bool {
        if let Some(sym) = self.symbols.get_mut(name) {
            if !sym.resolved {
                sym.offset = offset;
                sym.module = module.to_string();
                sym.resolved = true;
                return true;
            }
        }
        false
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    pub fn is_resolved(&self, name: &str) -> bool {
        self.symbols.get(name).map(|s| s.resolved).unwrap_or(false)
    }

    /// 收集所有未解析的外部符号
    pub fn unresolved(&self) -> Vec<&Symbol> {
        self.symbols.values().filter(|s| !s.resolved).collect()
    }

    /// 收集特定模块的所有公开符号
    pub fn public_symbols_of(&self, module: &str) -> Vec<&Symbol> {
        self.symbols.values()
            .filter(|s| s.module == module && s.visibility == SymbolVisibility::Public)
            .collect()
    }

    /// 合并另一个符号表（链接时合并）
    pub fn merge(&mut self, other: SymbolTable) -> Vec<String> {
        let mut errors = vec![];
        for (name, sym) in other.symbols {
            if let Err(e) = self.define(sym) {
                errors.push(e);
            }
        }
        errors
    }

    /// 验证所有外部引用已解析
    pub fn verify_all_resolved(&self) -> bool {
        let unresolved = self.unresolved();
        if !unresolved.is_empty() {
            for sym in &unresolved {
                eprintln!("未解析符号: `{}`", sym.name);
            }
            return false;
        }
        true
    }

    /// 生成符号表摘要报告
    pub fn summary(&self) -> SymbolTableSummary {
        let funcs = self.symbols.values().filter(|s| s.kind == SymbolKind::Function && s.resolved).count();
        let globals = self.symbols.values().filter(|s| s.kind == SymbolKind::Global && s.resolved).count();
        let unresolved = self.symbols.values().filter(|s| !s.resolved).count();
        let total_code: u64 = self.symbols.values()
            .filter(|s| s.kind == SymbolKind::Function)
            .map(|s| s.size as u64).sum();
        SymbolTableSummary { total: self.symbols.len(), funcs, globals, unresolved, total_code_bytes: total_code }
    }

    /// 按偏移排序导出所有函数符号（用于ELF符号表）
    pub fn sorted_functions(&self) -> Vec<&Symbol> {
        let mut funcs: Vec<&Symbol> = self.symbols.values()
            .filter(|s| s.kind == SymbolKind::Function && s.resolved)
            .collect();
        funcs.sort_by_key(|s| s.offset);
        funcs
    }

    /// 检查是否存在名称冲突（大小写不敏感）
    pub fn check_case_conflicts(&self) -> Vec<(String, String)> {
        let mut lower_map: HashMap<String, String> = HashMap::new();
        let mut conflicts = vec![];
        for name in self.symbols.keys() {
            let lower = name.to_lowercase();
            if let Some(existing) = lower_map.get(&lower) {
                if existing != name {
                    conflicts.push((existing.clone(), name.clone()));
                }
            } else {
                lower_map.insert(lower, name.clone());
            }
        }
        conflicts
    }
}

impl Default for SymbolTable {
    fn default() -> Self { Self::new() }
}

#[derive(Debug)]
pub struct SymbolTableSummary {
    pub total:            usize,
    pub funcs:            usize,
    pub globals:          usize,
    pub unresolved:       usize,
    pub total_code_bytes: u64,
}

impl SymbolTableSummary {
    pub fn format(&self) -> String {
        format!("符号表: {}个符号 ({}函数 {}全局 {}未解析) 代码:{:.1}KB",
            self.total, self.funcs, self.globals, self.unresolved,
            self.total_code_bytes as f64 / 1024.0)
    }
}
