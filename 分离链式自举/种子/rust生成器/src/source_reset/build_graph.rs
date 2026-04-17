// 构建图抽取
use std::path::Path;
use std::fs;

pub struct BuildGraph {
    pub units: Vec<BuildUnit>,
    pub external_deps: Vec<ExternalDep>,
}

#[allow(dead_code)] // 保留字段: 源码架构分析未来需要 (2026-04-17 A.4)
pub struct BuildUnit {
    pub name: String,
    pub kind: String,
    pub source_path: String,
}

#[allow(dead_code)] // 保留字段: 依赖分析未来需要 (2026-04-17 A.4)
pub struct ExternalDep {
    pub name: String,
    pub version: String,
}

pub fn extract_build_graph(path: &str, language: &str) -> BuildGraph {
    match language {
        "rust" => extract_cargo(path),
        "go" => extract_gomod(path),
        _ => BuildGraph { units: Vec::new(), external_deps: Vec::new() },
    }
}

fn extract_cargo(path: &str) -> BuildGraph {
    let mut graph = BuildGraph { units: Vec::new(), external_deps: Vec::new() };
    let cargo_path = Path::new(path).join("Cargo.toml");
    let content = match fs::read_to_string(&cargo_path) {
        Ok(c) => c,
        Err(_) => return graph,
    };

    // Extract package name
    let mut name = "unknown".to_string();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") && trimmed.contains('=') {
            if let Some(val) = trimmed.split('=').nth(1) {
                name = val.trim().trim_matches('"').to_string();
            }
        }
    }

    // Detect binary/lib
    if Path::new(path).join("src/main.rs").exists() {
        graph.units.push(BuildUnit {
            name: name.clone(),
            kind: "bin".into(),
            source_path: "src/main.rs".into(),
        });
    }
    if Path::new(path).join("src/lib.rs").exists() {
        graph.units.push(BuildUnit {
            name: name.clone(),
            kind: "lib".into(),
            source_path: "src/lib.rs".into(),
        });
    }

    // Extract dependencies
    let mut in_deps = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[dependencies]" { in_deps = true; continue; }
        if trimmed.starts_with('[') { in_deps = false; continue; }
        if in_deps && trimmed.contains('=') {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 {
                graph.external_deps.push(ExternalDep {
                    name: parts[0].trim().to_string(),
                    version: parts[1].trim().trim_matches('"').to_string(),
                });
            }
        }
    }
    graph
}

fn extract_gomod(path: &str) -> BuildGraph {
    let mut graph = BuildGraph { units: Vec::new(), external_deps: Vec::new() };
    let gomod_path = Path::new(path).join("go.mod");
    let content = match fs::read_to_string(&gomod_path) {
        Ok(c) => c,
        Err(_) => return graph,
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("module ") {
            let name = trimmed.strip_prefix("module ").unwrap_or("unknown");
            graph.units.push(BuildUnit {
                name: name.to_string(),
                kind: "module".into(),
                source_path: ".".into(),
            });
        }
    }
    graph
}
