// Go语言前端: 骨架解析
use std::path::Path;
use std::fs;
use super::semantic_facts::SkeletonResult;

pub fn parse_go_skeleton(path: &str) -> SkeletonResult {
    let mut result = SkeletonResult::empty();
    let p = Path::new(path);

    if let Ok(entries) = fs::read_dir(p) {
        for entry in entries.flatten() {
            let fp = entry.path();
            if fp.extension().map_or(false, |e| e == "go") {
                if let Ok(content) = fs::read_to_string(&fp) {
                    parse_go_file(&content, &mut result);
                }
            }
        }
    }
    result
}

fn parse_go_file(content: &str, result: &mut SkeletonResult) {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("func ") {
            if let Some(name) = extract_go_func_name(trimmed) {
                if name.chars().next().map_or(false, |c| c.is_uppercase()) {
                    result.public_apis.push(name.clone());
                }
                result.functions.push(name);
            }
        }
        if trimmed.starts_with("type ") {
            if let Some(name) = extract_go_type_name(trimmed) {
                result.types.push(name);
            }
        }
    }
}

fn extract_go_func_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("func ")?;
    // Method receiver: func (r *Type) Name(...)
    let rest = if rest.starts_with('(') {
        let close = rest.find(')')?;
        rest[close + 1..].trim_start()
    } else {
        rest
    };
    let end = rest.find(|c: char| c == '(' || c == ' ').unwrap_or(rest.len());
    if end > 0 { Some(rest[..end].to_string()) } else { None }
}

fn extract_go_type_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("type ")?.trim();
    let end = rest.find(|c: char| c == ' ' || c == '{').unwrap_or(rest.len());
    if end > 0 { Some(rest[..end].to_string()) } else { None }
}
