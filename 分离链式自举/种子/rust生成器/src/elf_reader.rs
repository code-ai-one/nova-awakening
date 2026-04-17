#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova ELF 文件读取/分析
/// 读取已生成的 ELF 二进制文件，提取节/符号/入口点等信息
/// 应用：自举验证 / 固定点检测 / 调试

use std::path::Path;

/// ELF 文件类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElfType { Exec, Dyn, Rel, Core, Unknown }

/// ELF 机器类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElfMachine { X86_64, Arm64, RiscV64, Unknown }

/// ELF 节描述
#[derive(Debug, Clone)]
pub struct ElfSection {
    pub name:     String,
    pub offset:   u64,
    pub size:     u64,
    pub vaddr:    u64,
    pub flags:    u64,    // SHF_WRITE=1, SHF_ALLOC=2, SHF_EXECINSTR=4
    pub sh_type:  u32,
}

impl ElfSection {
    pub fn is_executable(&self) -> bool { self.flags & 4 != 0 }
    pub fn is_writable(&self) -> bool { self.flags & 1 != 0 }
    pub fn is_alloc(&self) -> bool { self.flags & 2 != 0 }
}

/// ELF 符号表项
#[derive(Debug, Clone)]
pub struct ElfSymbol {
    pub name:    String,
    pub value:   u64,    // 地址/偏移
    pub size:    u64,
    pub bind:    u8,     // 0=local, 1=global, 2=weak
    pub sym_type: u8,   // 0=notype, 1=object, 2=func, 3=section, 4=file
    pub section: u16,
}

impl ElfSymbol {
    pub fn is_function(&self) -> bool { self.sym_type == 2 }
    pub fn is_global(&self) -> bool { self.bind == 1 }
    pub fn is_defined(&self) -> bool { self.section != 0 }
}

/// ELF 文件解析结果
#[derive(Debug)]
pub struct ElfInfo {
    pub file_type: ElfType,
    pub machine:   ElfMachine,
    pub entry:     u64,
    pub sections:  Vec<ElfSection>,
    pub symbols:   Vec<ElfSymbol>,
    pub file_size: usize,
}

impl ElfInfo {
    /// 解析 ELF 文件
    pub fn parse(path: &Path) -> Result<Self, String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("读取ELF失败 {}: {}", path.display(), e))?;
        Self::parse_bytes(&data)
    }

    pub fn parse_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 64 { return Err("文件太小（非ELF）".into()); }
        // ELF magic
        if &data[0..4] != b"\x7fELF" { return Err("非ELF文件（magic不匹配）".into()); }
        let class = data[4];   // 2 = 64位
        let endian = data[5];  // 1 = little, 2 = big
        if class != 2 { return Err("仅支持64位ELF".into()); }
        if endian != 1 { return Err("仅支持小端ELF".into()); }

        let read_u16 = |off: usize| -> u16 { u16::from_le_bytes([data[off], data[off+1]]) };
        let read_u32 = |off: usize| -> u32 { u32::from_le_bytes(data[off..off+4].try_into().unwrap_or([0;4])) };
        let read_u64 = |off: usize| -> u64 { u64::from_le_bytes(data[off..off+8].try_into().unwrap_or([0;8])) };

        let e_type = read_u16(16);
        let e_machine = read_u16(18);
        let e_entry = read_u64(24);
        let e_shoff = read_u64(40) as usize;  // 节头表偏移
        let e_shentsize = read_u16(58) as usize;
        let e_shnum = read_u16(60) as usize;
        let e_shstrndx = read_u16(62) as usize;

        let file_type = match e_type { 2 => ElfType::Exec, 3 => ElfType::Dyn, 1 => ElfType::Rel, 4 => ElfType::Core, _ => ElfType::Unknown };
        let machine = match e_machine { 62 => ElfMachine::X86_64, 183 => ElfMachine::Arm64, 243 => ElfMachine::RiscV64, _ => ElfMachine::Unknown };

        // 读取节名字符串表
        let mut shstrtab: &[u8] = b"";
        if e_shstrndx < e_shnum && e_shoff + (e_shstrndx + 1) * e_shentsize <= data.len() {
            let soff = e_shoff + e_shstrndx * e_shentsize;
            let sh_offset = read_u32(soff + 16) as usize;
            let sh_size = read_u32(soff + 20) as usize;
            if sh_offset + sh_size <= data.len() {
                shstrtab = &data[sh_offset..sh_offset + sh_size];
            }
        }

        let read_cstr = |tab: &[u8], off: usize| -> String {
            if off >= tab.len() { return String::new(); }
            let end = tab[off..].iter().position(|&b| b == 0).unwrap_or(tab.len() - off);
            String::from_utf8_lossy(&tab[off..off + end]).to_string()
        };

        // 读取所有节
        let mut sections = vec![];
        let mut symtab_off = 0usize;
        let mut symtab_size = 0usize;
        let mut strtab_data: &[u8] = b"";

        for i in 0..e_shnum {
            let soff = e_shoff + i * e_shentsize;
            if soff + e_shentsize > data.len() { break; }
            let sh_name = read_u32(soff) as usize;
            let sh_type = read_u32(soff + 4);
            let sh_flags = read_u64(soff + 8);
            let sh_addr  = read_u64(soff + 16);
            let sh_off   = read_u32(soff + 24) as u64;
            let sh_size  = read_u32(soff + 28) as u64;
            let name = read_cstr(shstrtab, sh_name);

            if sh_type == 2 {  // SHT_SYMTAB
                symtab_off = sh_off as usize;
                symtab_size = sh_size as usize;
            }
            if name == ".strtab" {
                if sh_off as usize + sh_size as usize <= data.len() {
                    strtab_data = &data[sh_off as usize..sh_off as usize + sh_size as usize];
                }
            }
            sections.push(ElfSection { name, offset: sh_off, size: sh_size, vaddr: sh_addr, flags: sh_flags, sh_type });
        }

        // 读取符号表
        let mut symbols = vec![];
        if symtab_off > 0 && symtab_off + symtab_size <= data.len() {
            let sym_entry_size = 24usize;  // Elf64_Sym = 24 bytes
            let mut off = symtab_off;
            while off + sym_entry_size <= symtab_off + symtab_size {
                let st_name  = read_u32(off) as usize;
                let st_info  = data[off + 4];
                let st_shndx = read_u16(off + 6);
                let st_value = read_u64(off + 8);
                let st_size  = read_u64(off + 16);
                let name = read_cstr(strtab_data, st_name);
                symbols.push(ElfSymbol {
                    name, value: st_value, size: st_size,
                    bind: st_info >> 4, sym_type: st_info & 0xF, section: st_shndx,
                });
                off += sym_entry_size;
            }
        }

        Ok(ElfInfo { file_type, machine, entry: e_entry, sections, symbols, file_size: data.len() })
    }

    /// 查找特定名称的节
    pub fn find_section(&self, name: &str) -> Option<&ElfSection> {
        self.sections.iter().find(|s| s.name == name)
    }

    /// 查找特定名称的符号
    pub fn find_symbol(&self, name: &str) -> Option<&ElfSymbol> {
        self.symbols.iter().find(|s| s.name == name)
    }

    /// 代码节大小
    pub fn code_size(&self) -> u64 {
        self.sections.iter().filter(|s| s.is_executable()).map(|s| s.size).sum()
    }

    /// 数据节大小
    pub fn data_size(&self) -> u64 {
        self.sections.iter().filter(|s| s.is_alloc() && !s.is_executable()).map(|s| s.size).sum()
    }

    /// 格式化摘要
    pub fn summary(&self) -> String {
        format!(
            "ELF {:?} {:?}: 入口={:#x} 节:{} 符号:{} 代码:{:.1}KB 数据:{:.1}KB 总:{:.1}KB",
            self.file_type, self.machine, self.entry,
            self.sections.len(), self.symbols.len(),
            self.code_size() as f64 / 1024.0,
            self.data_size() as f64 / 1024.0,
            self.file_size as f64 / 1024.0
        )
    }

    /// 比较两个 ELF 的差异（用于自举固定点检测）
    pub fn diff(&self, other: &ElfInfo) -> ElfDiff {
        ElfDiff {
            size_diff: other.file_size as i64 - self.file_size as i64,
            code_diff: other.code_size() as i64 - self.code_size() as i64,
            symbol_count_diff: other.symbols.len() as i64 - self.symbols.len() as i64,
            entry_same: self.entry == other.entry,
            identical_bytes: self.file_size == other.file_size,
        }
    }
}

#[derive(Debug)]
pub struct ElfDiff {
    pub size_diff:         i64,
    pub code_diff:         i64,
    pub symbol_count_diff: i64,
    pub entry_same:        bool,
    pub identical_bytes:   bool,
}
impl ElfDiff {
    pub fn is_fixed_point(&self) -> bool { self.identical_bytes }
    pub fn format(&self) -> String {
        if self.identical_bytes {
            "✓ 固定点（两个ELF完全一致）".into()
        } else {
            format!("✗ 非固定点: 大小差{}B 代码差{}B 符号数差{}",
                self.size_diff, self.code_diff, self.symbol_count_diff)
        }
    }
}
