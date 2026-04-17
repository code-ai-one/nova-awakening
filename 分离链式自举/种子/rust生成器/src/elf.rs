use std::path::PathBuf;

const FILE_ALIGN: usize = 0x1000;
const SEGMENT_ALIGN: usize = 0x1000;
const IMAGE_BASE: u64 = 0x400000;

#[derive(Debug, Clone)]
pub struct StageArtifacts {
    pub workspace_root: PathBuf,
    pub stage0_root: PathBuf,
    pub output_root: PathBuf,
    pub stage1_binary: PathBuf,
    pub stage2_binary: PathBuf,
    pub stage3_binary: PathBuf,
}

impl StageArtifacts {
    pub fn new(workspace_root: PathBuf) -> Self {
        let stage0_root = workspace_root.join("阶段0_种子编译器");
        let output_root = workspace_root.join("阶段1_自举启动");
        let stage1_binary = output_root.join(default_binary_name("阶段1_编译器", OutputTarget::Linux));
        let stage2_binary = output_root.join(default_binary_name("阶段2_编译器", OutputTarget::Linux));
        let stage3_binary = output_root.join(default_binary_name("阶段3_编译器", OutputTarget::Linux));

        Self {
            workspace_root,
            stage0_root,
            output_root,
            stage1_binary,
            stage2_binary,
            stage3_binary,
        }
    }

    pub fn stage0_cache_dir(&self) -> PathBuf {
        self.stage0_root.join(".nova_cache")
    }

    pub fn describe(&self) -> Vec<(String, String)> {
        vec![
            ("workspace_root".to_string(), self.workspace_root.display().to_string()),
            ("stage0_root".to_string(), self.stage0_root.display().to_string()),
            ("output_root".to_string(), self.output_root.display().to_string()),
            ("stage1_binary".to_string(), self.stage1_binary.display().to_string()),
            ("stage2_binary".to_string(), self.stage2_binary.display().to_string()),
            ("stage3_binary".to_string(), self.stage3_binary.display().to_string()),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputTarget {
    Linux,
}

pub fn default_binary_name(base: &str, target: OutputTarget) -> String {
    match target {
        OutputTarget::Linux => base.to_string(),
    }
}

#[allow(dead_code)] // 保留code_vaddr/data_file_off字段: ELF布局元信息未来诊断/调试需要 (2026-04-17 A.4)
pub struct ElfResult {
    pub data: Vec<u8>,
    pub code_vaddr: u64,
    pub data_vaddr: u64,
    pub data_file_off: usize,
}

fn write_u16(buf: &mut [u8], off: usize, value: u16) {
    buf[off..off + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(buf: &mut [u8], off: usize, value: u32) {
    buf[off..off + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(buf: &mut [u8], off: usize, value: u64) {
    buf[off..off + 8].copy_from_slice(&value.to_le_bytes());
}

pub fn build_elf(code: &[u8]) -> ElfResult {
    let code_size = code.len();
    let code_file_off = FILE_ALIGN;
    let code_vaddr = IMAGE_BASE + code_file_off as u64;
    let code_aligned = code_size.div_ceil(FILE_ALIGN) * FILE_ALIGN;
    let data_file_off = code_file_off + code_aligned;
    let data_vaddr = IMAGE_BASE + data_file_off as u64;
    let data_size = FILE_ALIGN;
    let ehdr_size = 64usize;
    let phdr_size = 56usize;
    let phnum = 2usize;
    let total_size = data_file_off + data_size;
    let mut elf = vec![0u8; total_size];

    elf[0] = 0x7F;
    elf[1] = b'E';
    elf[2] = b'L';
    elf[3] = b'F';
    elf[4] = 2;
    elf[5] = 1;
    elf[6] = 1;
    elf[7] = 0;

    write_u16(&mut elf, 16, 2);
    write_u16(&mut elf, 18, 0x3E);
    write_u32(&mut elf, 20, 1);
    write_u64(&mut elf, 24, code_vaddr);
    write_u64(&mut elf, 32, ehdr_size as u64);
    write_u64(&mut elf, 40, 0);
    write_u32(&mut elf, 48, 0);
    write_u16(&mut elf, 52, ehdr_size as u16);
    write_u16(&mut elf, 54, phdr_size as u16);
    write_u16(&mut elf, 56, phnum as u16);
    write_u16(&mut elf, 58, 0x40);
    write_u16(&mut elf, 60, 0);
    write_u16(&mut elf, 62, 0);

    let ph1 = ehdr_size;
    write_u32(&mut elf, ph1, 1);
    write_u32(&mut elf, ph1 + 4, 7);
    write_u64(&mut elf, ph1 + 8, 0);
    write_u64(&mut elf, ph1 + 16, IMAGE_BASE);
    write_u64(&mut elf, ph1 + 24, IMAGE_BASE);
    write_u64(&mut elf, ph1 + 32, (code_file_off + code_size) as u64);
    write_u64(&mut elf, ph1 + 40, (code_file_off + code_size) as u64);
    write_u64(&mut elf, ph1 + 48, SEGMENT_ALIGN as u64);

    let ph2 = ehdr_size + phdr_size;
    write_u32(&mut elf, ph2, 1);
    write_u32(&mut elf, ph2 + 4, 6);
    write_u64(&mut elf, ph2 + 8, data_file_off as u64);
    write_u64(&mut elf, ph2 + 16, data_vaddr);
    write_u64(&mut elf, ph2 + 24, data_vaddr);
    write_u64(&mut elf, ph2 + 32, data_size as u64);
    write_u64(&mut elf, ph2 + 40, data_size as u64 + 0x1000_0000);
    write_u64(&mut elf, ph2 + 48, SEGMENT_ALIGN as u64);

    elf[code_file_off..code_file_off + code_size].copy_from_slice(code);

    ElfResult {
        data: elf,
        code_vaddr,
        data_vaddr,
        data_file_off,
    }
}
