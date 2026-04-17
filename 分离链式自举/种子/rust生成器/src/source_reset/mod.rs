// ═══════════════════════════════════════════════════════════════
// Nova · Rust Helper · 跨语言原生重置模块
// 职责: 辅助Nova主链完成外源工程的探测、解析、语义抽取
// ═══════════════════════════════════════════════════════════════

pub mod detect;
pub mod source_contract;
pub mod build_graph;
pub mod rust_frontend;
pub mod go_frontend;
pub mod c_frontend;
pub mod semantic_facts;
pub mod compliance;

/// 跨语言原生重置主入口
pub fn run_source_reset(project_path: &str) -> SourceResetReport {
    let mut report = SourceResetReport::new(project_path);

    // Step 1: 工程探测
    let detection = detect::detect_project(project_path);
    report.language = detection.language.clone();
    report.build_system = detection.build_system.clone();
    report.entries = detection.entries.clone();

    // Step 2: 构建图抽取
    let build = build_graph::extract_build_graph(project_path, &detection.language);
    report.build_units = build.units.len();
    report.external_deps = build.external_deps.len();

    // Step 3: 语言骨架解析
    let skeleton = match detection.language.as_str() {
        "rust" => rust_frontend::parse_rust_skeleton(project_path),
        "go" => go_frontend::parse_go_skeleton(project_path),
        "c" | "cpp" => c_frontend::parse_c_skeleton(project_path),
        _ => semantic_facts::SkeletonResult::empty(),
    };
    report.public_apis = skeleton.public_apis.len();
    report.types = skeleton.types.len();
    report.functions = skeleton.functions.len();

    // Step 4: 合规检查
    let license = compliance::check_licenses(project_path);
    report.license_compatible = license.all_compatible;
    report.license_details = license.details.clone();

    report
}

/// 重置报告
pub struct SourceResetReport {
    pub project_path: String,
    pub language: String,
    pub build_system: String,
    pub entries: Vec<String>,
    pub build_units: usize,
    pub external_deps: usize,
    pub public_apis: usize,
    pub types: usize,
    pub functions: usize,
    pub license_compatible: bool,
    pub license_details: Vec<String>,
}

impl SourceResetReport {
    pub fn new(path: &str) -> Self {
        Self {
            project_path: path.to_string(),
            language: String::new(),
            build_system: String::new(),
            entries: Vec::new(),
            build_units: 0,
            external_deps: 0,
            public_apis: 0,
            types: 0,
            functions: 0,
            license_compatible: true,
            license_details: Vec::new(),
        }
    }

    pub fn print_summary(&self) {
        eprintln!("═══ Source Reset Report ═══");
        eprintln!("  Project: {}", self.project_path);
        eprintln!("  Language: {}", self.language);
        eprintln!("  Build System: {}", self.build_system);
        eprintln!("  Entries: {}", self.entries.len());
        eprintln!("  Build Units: {}", self.build_units);
        eprintln!("  External Deps: {}", self.external_deps);
        eprintln!("  Public APIs: {}", self.public_apis);
        eprintln!("  Types: {}", self.types);
        eprintln!("  Functions: {}", self.functions);
        eprintln!("  License Compatible: {}", self.license_compatible);
    }
}
