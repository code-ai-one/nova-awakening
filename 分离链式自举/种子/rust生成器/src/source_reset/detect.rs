// 工程探测: 识别语言类型和构建系统
use std::path::Path;

pub struct DetectionResult {
    pub language: String,
    pub build_system: String,
    pub workspace_root: String,
    pub package_roots: Vec<String>,
    pub entries: Vec<String>,
}

pub fn detect_project(path: &str) -> DetectionResult {
    let mut result = DetectionResult {
        language: String::new(),
        build_system: String::new(),
        workspace_root: path.to_string(),
        package_roots: Vec::new(),
        entries: Vec::new(),
    };

    let p = Path::new(path);

    // Rust: Cargo.toml
    if p.join("Cargo.toml").exists() {
        result.language = "rust".into();
        result.build_system = "cargo".into();
        if p.join("src/main.rs").exists() {
            result.entries.push("src/main.rs".into());
        }
        if p.join("src/lib.rs").exists() {
            result.entries.push("src/lib.rs".into());
        }
        result.package_roots.push(path.into());
        return result;
    }

    // Go: go.mod
    if p.join("go.mod").exists() {
        result.language = "go".into();
        result.build_system = "go".into();
        if p.join("main.go").exists() {
            result.entries.push("main.go".into());
        }
        result.package_roots.push(path.into());
        return result;
    }

    // Python: pyproject.toml / setup.py
    if p.join("pyproject.toml").exists() || p.join("setup.py").exists() {
        result.language = "python".into();
        result.build_system = "pyproject".into();
        return result;
    }

    // Java: pom.xml / build.gradle
    if p.join("pom.xml").exists() {
        result.language = "java".into();
        result.build_system = "maven".into();
        return result;
    }
    if p.join("build.gradle").exists() || p.join("build.gradle.kts").exists() {
        result.language = "java".into();
        result.build_system = "gradle".into();
        return result;
    }

    // C/C++: CMakeLists.txt / Makefile
    if p.join("CMakeLists.txt").exists() {
        result.language = "c".into();
        result.build_system = "cmake".into();
        return result;
    }
    if p.join("Makefile").exists() {
        result.language = "c".into();
        result.build_system = "make".into();
        return result;
    }

    result.language = "unknown".into();
    result.build_system = "unknown".into();
    result
}
