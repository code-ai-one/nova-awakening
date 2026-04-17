// SourceProjectContract: 外源工程合同数据结构

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SourceProjectContract {
    pub name: String,
    pub source_language: String,
    pub source_build_system: String,
    pub workspace_root: String,
    pub package_roots: Vec<String>,
    pub entrypoints: Vec<String>,
    pub public_api: Vec<String>,
    pub memory_model: MemoryModel,
    pub concurrency_model: ConcurrencyModel,
    pub error_model: ErrorModel,
    pub ffi_boundaries: Vec<String>,
    pub high_risk_items: Vec<String>,
    pub license_inputs: Vec<LicenseInput>,
    pub reset_profile: ResetProfile,
    pub unsupported_items: Vec<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum MemoryModel {
    Ownership,
    GC,
    Manual,
    Hybrid,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum ConcurrencyModel {
    Threads,
    Goroutine,
    AsyncIO,
    JvmThread,
    EventLoop,
    None,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum ErrorModel {
    Result,
    Panic,
    Exception,
    Errno,
    Mixed,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum ResetProfile {
    StrictNative,
    HostedCompatible,
    HybridBridge,
}

#[allow(dead_code)] // 保留字段: 许可证审计未来需要 (2026-04-17 A.4)
#[derive(Clone, Debug)]
pub struct LicenseInput {
    pub name: String,
    pub license_type: String,
    pub compatible: bool,
}

#[allow(dead_code)]
impl SourceProjectContract {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            source_language: String::new(),
            source_build_system: String::new(),
            workspace_root: String::new(),
            package_roots: Vec::new(),
            entrypoints: Vec::new(),
            public_api: Vec::new(),
            memory_model: MemoryModel::GC,
            concurrency_model: ConcurrencyModel::None,
            error_model: ErrorModel::Mixed,
            ffi_boundaries: Vec::new(),
            high_risk_items: Vec::new(),
            license_inputs: Vec::new(),
            reset_profile: ResetProfile::HostedCompatible,
            unsupported_items: Vec::new(),
        }
    }

    pub fn from_detection(det: &super::detect::DetectionResult) -> Self {
        let mut c = Self::new();
        c.source_language = det.language.clone();
        c.source_build_system = det.build_system.clone();
        c.workspace_root = det.workspace_root.clone();
        c.package_roots = det.package_roots.clone();
        c.entrypoints = det.entries.clone();

        match det.language.as_str() {
            "rust" => {
                c.memory_model = MemoryModel::Ownership;
                c.concurrency_model = ConcurrencyModel::AsyncIO;
                c.error_model = ErrorModel::Result;
            }
            "go" => {
                c.memory_model = MemoryModel::GC;
                c.concurrency_model = ConcurrencyModel::Goroutine;
                c.error_model = ErrorModel::Panic;
            }
            "c" => {
                c.memory_model = MemoryModel::Manual;
                c.concurrency_model = ConcurrencyModel::Threads;
                c.error_model = ErrorModel::Errno;
            }
            "cpp" => {
                c.memory_model = MemoryModel::Hybrid;
                c.concurrency_model = ConcurrencyModel::Threads;
                c.error_model = ErrorModel::Exception;
            }
            "python" => {
                c.memory_model = MemoryModel::GC;
                c.concurrency_model = ConcurrencyModel::AsyncIO;
                c.error_model = ErrorModel::Exception;
            }
            "java" => {
                c.memory_model = MemoryModel::GC;
                c.concurrency_model = ConcurrencyModel::JvmThread;
                c.error_model = ErrorModel::Exception;
            }
            _ => {}
        }
        c
    }
}
