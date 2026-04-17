// Rust语言前端: 骨架解析
use std::path::Path;
use std::fs;
use super::semantic_facts::SkeletonResult;

pub fn parse_rust_skeleton(path: &str) -> SkeletonResult {
    let mut result = SkeletonResult::empty();
    let src_dir = Path::new(path).join("src");
    if !src_dir.exists() { return result; }

    if let Ok(entries) = fs::read_dir(&src_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map_or(false, |e| e == "rs") {
                if let Ok(content) = fs::read_to_string(&p) {
                    parse_rust_file(&content, &mut result);
                }
            }
        }
    }
    result
}

fn parse_rust_file(content: &str, result: &mut SkeletonResult) {
    for line in content.lines() {
        let trimmed = line.trim();
        // Public functions
        if trimmed.starts_with("pub fn ") || trimmed.starts_with("pub(crate) fn ") {
            if let Some(name) = extract_fn_name(trimmed) {
                result.public_apis.push(name.clone());
                result.functions.push(name);
            }
        } else if trimmed.starts_with("fn ") {
            if let Some(name) = extract_fn_name(trimmed) {
                result.functions.push(name);
            }
        }
        // Structs
        if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
            if let Some(name) = extract_type_name(trimmed, "struct ") {
                result.types.push(name);
            }
        }
        // Enums
        if trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ") {
            if let Some(name) = extract_type_name(trimmed, "enum ") {
                result.types.push(name);
            }
        }
        // Traits
        if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
            if let Some(name) = extract_type_name(trimmed, "trait ") {
                result.types.push(name);
            }
        }
    }
}

fn extract_fn_name(line: &str) -> Option<String> {
    let idx = line.find("fn ")?;
    let rest = &line[idx + 3..];
    let end = rest.find(|c: char| c == '(' || c == '<' || c == ' ')?;
    Some(rest[..end].to_string())
}

fn extract_type_name(line: &str, keyword: &str) -> Option<String> {
    let idx = line.find(keyword)?;
    let rest = &line[idx + keyword.len()..];
    let end = rest.find(|c: char| c == ' ' || c == '{' || c == '(' || c == '<' || c == ';')
        .unwrap_or(rest.len());
    if end > 0 { Some(rest[..end].to_string()) } else { None }
}
