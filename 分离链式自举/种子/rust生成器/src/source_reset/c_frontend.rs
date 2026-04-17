// C/C++语言前端: 骨架解析
use std::path::Path;
use std::fs;
use super::semantic_facts::SkeletonResult;

pub fn parse_c_skeleton(path: &str) -> SkeletonResult {
    let mut result = SkeletonResult::empty();
    let p = Path::new(path);

    // Scan src/ and root for .c/.h/.cpp/.hpp files
    for dir in &["src", "."] {
        let d = p.join(dir);
        if let Ok(entries) = fs::read_dir(&d) {
            for entry in entries.flatten() {
                let fp = entry.path();
                let ext = fp.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "c" | "h" | "cpp" | "hpp" | "cc") {
                    if let Ok(content) = fs::read_to_string(&fp) {
                        parse_c_file(&content, &mut result);
                    }
                }
            }
        }
    }
    result
}

fn parse_c_file(content: &str, result: &mut SkeletonResult) {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("struct ") {
            if let Some(name) = extract_word(trimmed, "struct ") {
                result.types.push(name);
            }
        }
        if trimmed.starts_with("enum ") {
            if let Some(name) = extract_word(trimmed, "enum ") {
                result.types.push(name);
            }
        }
        if trimmed.starts_with("typedef ") {
            result.types.push(trimmed.to_string());
        }
        if trimmed.starts_with("#define ") {
            if let Some(name) = extract_word(trimmed, "#define ") {
                result.functions.push(format!("MACRO_{}", name));
            }
        }
    }
}

fn extract_word(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?.trim();
    let end = rest.find(|c: char| c == ' ' || c == '{' || c == ';' || c == '(')
        .unwrap_or(rest.len());
    if end > 0 { Some(rest[..end].to_string()) } else { None }
}
