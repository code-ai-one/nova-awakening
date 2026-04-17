#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 目标文件写入器 (Object File Writer)
/// 生成 ELF64 目标文件（.o），配合 linker.rs 使用
/// 支持：代码节/.text / 数据节/.data / 只读数据/.rodata / 符号表 / 重定位表

use std::collections::HashMap;
use std::path::Path;

// ELF64 魔数和常量
const ELF_MAGIC:    [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELFCLASS64:   u8 = 2;
const ELFDATA2LSB:  u8 = 1;  // 小端序
const ET_REL:       u16 = 1;  // 可重定位文件
const EM_X86_64:    u16 = 62;
const EM_AARCH64:   u16 = 183;
const EM_RISCV:     u16 = 243;
const EV_CURRENT:   u8 = 1;

// ELF 节类型
const SHT_NULL:     u32 = 0;
const SHT_PROGBITS: u32 = 1;
const SHT_SYMTAB:   u32 = 2;
const SHT_STRTAB:   u32 = 3;
const SHT_RELA:     u32 = 4;

// ELF 节标志
const SHF_WRITE:    u64 = 1;
const SHF_ALLOC:    u64 = 2;
const SHF_EXECINSTR: u64 = 4;

// ELF 符号类型
const STT_NOTYPE:   u8 = 0;
const STT_FUNC:     u8 = 2;
const STT_OBJECT:   u8 = 1;

// ELF 符号绑定
const STB_LOCAL:    u8 = 0;
const STB_GLOBAL:   u8 = 1;

// 重定位类型
const R_X86_64_PLT32:  u32 = 4;
const R_X86_64_PC32:   u32 = 2;
const R_X86_64_32:     u32 = 10;
const R_X86_64_64:     u32 = 1;

/// 节描述
#[derive(Debug, Clone)]
pub struct Section {
    pub name:   String,
    pub data:   Vec<u8>,
    pub kind:   SectionKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SectionKind { Text, Data, Rodata, Bss }

impl Section {
    pub fn text(data: Vec<u8>)   -> Self { Section { name: ".text".into(), data, kind: SectionKind::Text } }
    pub fn data(data: Vec<u8>)   -> Self { Section { name: ".data".into(), data, kind: SectionKind::Data } }
    pub fn rodata(data: Vec<u8>) -> Self { Section { name: ".rodata".into(), data, kind: SectionKind::Rodata } }
    pub fn custom(name: impl Into<String>, data: Vec<u8>, kind: SectionKind) -> Self {
        Section { name: name.into(), data, kind }
    }
    pub fn flags(&self) -> u64 {
        match self.kind {
            SectionKind::Text   => SHF_ALLOC | SHF_EXECINSTR,
            SectionKind::Data   => SHF_ALLOC | SHF_WRITE,
            SectionKind::Rodata => SHF_ALLOC,
            SectionKind::Bss    => SHF_ALLOC | SHF_WRITE,
        }
    }
    pub fn sh_type(&self) -> u32 { SHT_PROGBITS }
    pub fn align(&self) -> u64 {
        match self.kind {
            SectionKind::Text => 16, SectionKind::Rodata => 8, _ => 8,
        }
    }
}

/// 符号
#[derive(Debug, Clone)]
pub struct ObjSymbol {
    pub name:    String,
    pub value:   u64,     // 符号在节中的偏移
    pub size:    u64,
    pub section: Option<usize>,  // 所属节的索引（None = 未定义）
    pub kind:    SymKind,
    pub global:  bool,
}

#[derive(Debug, Clone, Copy)]
pub enum SymKind { Func, Data, Undef }

/// 重定位条目（RELA）
#[derive(Debug, Clone)]
pub struct Reloc {
    pub offset:   u64,    // 在节中的偏移
    pub sym_idx:  u32,    // 引用的符号索引
    pub rela_type: u32,   // 重定位类型
    pub addend:   i64,
}

/// ELF64 目标文件构建器
pub struct ObjectWriter {
    pub sections: Vec<Section>,
    pub symbols:  Vec<ObjSymbol>,
    pub relocs:   Vec<(usize, Vec<Reloc>)>,  // (section_idx, relocs)
    machine:      u16,
}

impl ObjectWriter {
    pub fn new_x86_64()  -> Self { ObjectWriter { sections: vec![], symbols: vec![], relocs: vec![], machine: EM_X86_64 } }
    pub fn new_aarch64() -> Self { ObjectWriter { sections: vec![], symbols: vec![], relocs: vec![], machine: EM_AARCH64 } }
    pub fn new_riscv64() -> Self { ObjectWriter { sections: vec![], symbols: vec![], relocs: vec![], machine: EM_RISCV } }

    pub fn add_section(&mut self, sec: Section) -> usize {
        let i = self.sections.len();
        self.sections.push(sec);
        i
    }

    pub fn add_symbol(&mut self, sym: ObjSymbol) -> usize {
        let i = self.symbols.len();
        self.symbols.push(sym);
        i
    }

    pub fn add_func_symbol(&mut self, name: impl Into<String>, sec_idx: usize, offset: u64, size: u64, global: bool) -> usize {
        self.add_symbol(ObjSymbol { name: name.into(), value: offset, size, section: Some(sec_idx), kind: SymKind::Func, global })
    }

    pub fn add_reloc(&mut self, sec_idx: usize, r: Reloc) {
        if let Some(entry) = self.relocs.iter_mut().find(|(i, _)| *i == sec_idx) {
            entry.1.push(r);
        } else {
            self.relocs.push((sec_idx, vec![r]));
        }
    }

    /// 生成 ELF64 二进制
    pub fn emit(&self) -> Vec<u8> {
        let mut out = vec![];
        // 字符串表（节名 + 符号名）
        let mut shstrtab = vec![0u8];  // 空字符串 = 索引0
        let mut strtab = vec![0u8];    // 符号字符串表（第一个是空）

        let mut sec_name_idx = vec![];
        for sec in &self.sections {
            sec_name_idx.push(shstrtab.len() as u32);
            shstrtab.extend_from_slice(sec.name.as_bytes());
            shstrtab.push(0);
        }
        // 添加 .symtab .strtab .shstrtab 节名
        let symtab_name_idx = shstrtab.len() as u32;
        shstrtab.extend_from_slice(b".symtab\0");
        let strtab_name_idx = shstrtab.len() as u32;
        shstrtab.extend_from_slice(b".strtab\0");
        let shstrtab_name_idx = shstrtab.len() as u32;
        shstrtab.extend_from_slice(b".shstrtab\0");

        let mut sym_name_idx = vec![];
        for sym in &self.symbols {
            sym_name_idx.push(strtab.len() as u32);
            strtab.extend_from_slice(sym.name.as_bytes());
            strtab.push(0);
        }

        // ELF Header (64字节)
        let ehdr = self.elf_header();
        out.extend_from_slice(&ehdr);

        // 节数据（对齐）
        let data_start = out.len() as u64;
        let mut sec_offsets = vec![];
        for sec in &self.sections {
            let align = sec.align() as usize;
            while out.len() % align != 0 { out.push(0); }
            sec_offsets.push(out.len() as u64);
            out.extend_from_slice(&sec.data);
        }

        // 符号表
        while out.len() % 8 != 0 { out.push(0); }
        let symtab_offset = out.len() as u64;
        // 第一个符号必须是 STN_UNDEF
        out.extend_from_slice(&[0u8; 24]);
        for (i, sym) in self.symbols.iter().enumerate() {
            let bind = if sym.global { STB_GLOBAL } else { STB_LOCAL };
            let stype = match sym.kind { SymKind::Func => STT_FUNC, SymKind::Data => STT_OBJECT, SymKind::Undef => STT_NOTYPE };
            let info = (bind << 4) | stype;
            let sh_idx: u16 = sym.section.map(|s| s as u16 + 1).unwrap_or(0);
            let mut entry = vec![];
            entry.extend_from_slice(&sym_name_idx[i].to_le_bytes());
            entry.push(info);
            entry.push(0);  // other
            entry.extend_from_slice(&sh_idx.to_le_bytes());
            entry.extend_from_slice(&sym.value.to_le_bytes());
            entry.extend_from_slice(&sym.size.to_le_bytes());
            out.extend_from_slice(&entry);
        }
        let symtab_size = out.len() as u64 - symtab_offset;

        // 字符串表
        let strtab_offset = out.len() as u64;
        out.extend_from_slice(&strtab);
        let strtab_size = strtab.len() as u64;

        // shstrtab
        let shstrtab_offset = out.len() as u64;
        out.extend_from_slice(&shstrtab);
        let shstrtab_size = shstrtab.len() as u64;

        // 节头表
        while out.len() % 8 != 0 { out.push(0); }
        let shoff = out.len() as u64;
        let n_secs = self.sections.len() + 4;  // +4: NULL, symtab, strtab, shstrtab
        // NULL节头
        out.extend_from_slice(&[0u8; 64]);
        // 代码/数据节头
        for (i, sec) in self.sections.iter().enumerate() {
            out.extend_from_slice(&self.section_header(
                sec_name_idx[i], sec.sh_type(), sec.flags(), 0,
                sec_offsets[i], sec.data.len() as u64, 0, 0, sec.align(), 0
            ));
        }
        // symtab
        let n_local = self.symbols.iter().filter(|s| !s.global).count() + 1;
        out.extend_from_slice(&self.section_header(
            symtab_name_idx, SHT_SYMTAB, 0, 0,
            symtab_offset, symtab_size, (self.sections.len() + 2) as u32, n_local as u32, 8, 24
        ));
        // strtab
        out.extend_from_slice(&self.section_header(strtab_name_idx, SHT_STRTAB, 0, 0, strtab_offset, strtab_size, 0, 0, 1, 0));
        // shstrtab
        out.extend_from_slice(&self.section_header(shstrtab_name_idx, SHT_STRTAB, 0, 0, shstrtab_offset, shstrtab_size, 0, 0, 1, 0));

        // 回填 ELF header 中的 shoff 和 shnum
        let shnum = n_secs as u16;
        let shstrndx = (n_secs - 1) as u16;
        out[40..48].copy_from_slice(&shoff.to_le_bytes());
        out[60..62].copy_from_slice(&shnum.to_le_bytes());
        out[62..64].copy_from_slice(&shstrndx.to_le_bytes());
        out
    }

    fn elf_header(&self) -> [u8; 64] {
        let mut h = [0u8; 64];
        h[0..4].copy_from_slice(&ELF_MAGIC);
        h[4] = ELFCLASS64; h[5] = ELFDATA2LSB; h[6] = EV_CURRENT; h[7] = 0;
        h[16..18].copy_from_slice(&ET_REL.to_le_bytes());
        h[18..20].copy_from_slice(&self.machine.to_le_bytes());
        h[20..24].copy_from_slice(&1u32.to_le_bytes());  // e_version
        h[52..54].copy_from_slice(&64u16.to_le_bytes()); // e_ehsize
        h[54..56].copy_from_slice(&0u16.to_le_bytes());  // e_phentsize
        h[56..58].copy_from_slice(&0u16.to_le_bytes());  // e_phnum
        h[58..60].copy_from_slice(&64u16.to_le_bytes()); // e_shentsize
        h
    }

    fn section_header(&self, name: u32, sh_type: u32, flags: u64, addr: u64, offset: u64, size: u64, link: u32, info: u32, align: u64, entsize: u64) -> [u8; 64] {
        let mut h = [0u8; 64];
        h[0..4].copy_from_slice(&name.to_le_bytes());
        h[4..8].copy_from_slice(&sh_type.to_le_bytes());
        h[8..16].copy_from_slice(&flags.to_le_bytes());
        h[16..24].copy_from_slice(&addr.to_le_bytes());
        h[24..32].copy_from_slice(&offset.to_le_bytes());
        h[32..40].copy_from_slice(&size.to_le_bytes());
        h[40..44].copy_from_slice(&link.to_le_bytes());
        h[44..48].copy_from_slice(&info.to_le_bytes());
        h[48..56].copy_from_slice(&align.to_le_bytes());
        h[56..64].copy_from_slice(&entsize.to_le_bytes());
        h
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.emit()).map_err(|e| format!("写入目标文件失败: {}", e))
    }

    pub fn stats(&self) -> ObjStats {
        let code_size: usize = self.sections.iter().filter(|s| s.kind == SectionKind::Text).map(|s| s.data.len()).sum();
        let data_size: usize = self.sections.iter().filter(|s| s.kind != SectionKind::Text).map(|s| s.data.len()).sum();
        ObjStats { sections: self.sections.len(), symbols: self.symbols.len(), code_size, data_size }
    }
}

#[derive(Debug)]
pub struct ObjStats {
    pub sections:  usize,
    pub symbols:   usize,
    pub code_size: usize,
    pub data_size: usize,
}
impl ObjStats {
    pub fn format(&self) -> String {
        format!("目标文件: {}个节 {}个符号 代码{:.1}KB 数据{:.1}KB",
            self.sections, self.symbols, self.code_size as f64 / 1024.0, self.data_size as f64 / 1024.0)
    }
}
