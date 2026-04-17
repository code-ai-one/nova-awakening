// 合规检查: 许可证兼容性验证
use std::path::Path;
use std::fs;

pub struct ComplianceResult {
    pub all_compatible: bool,
    pub details: Vec<String>,
}

pub fn check_licenses(path: &str) -> ComplianceResult {
    let mut result = ComplianceResult {
        all_compatible: true,
        details: Vec::new(),
    };

    let p = Path::new(path);

    // Check for LICENSE file
    for name in &["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING"] {
        let lp = p.join(name);
        if lp.exists() {
            if let Ok(content) = fs::read_to_string(&lp) {
                let license_type = detect_license_type(&content);
                let compatible = is_compatible(&license_type);
                result.details.push(format!("{}: {} (compatible={})", name, license_type, compatible));
                if !compatible {
                    result.all_compatible = false;
                }
            }
        }
    }

    // Check Cargo.toml for license field
    let cargo_path = p.join("Cargo.toml");
    if cargo_path.exists() {
        if let Ok(content) = fs::read_to_string(&cargo_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("license") && trimmed.contains('=') {
                    if let Some(val) = trimmed.split('=').nth(1) {
                        let license = val.trim().trim_matches('"');
                        result.details.push(format!("Cargo.toml license: {}", license));
                    }
                }
            }
        }
    }

    if result.details.is_empty() {
        result.details.push("No license file found - manual review required".into());
        result.all_compatible = false;
    }

    result
}

fn detect_license_type(content: &str) -> String {
    let lower = content.to_lowercase();
    if lower.contains("mit license") || lower.contains("permission is hereby granted, free of charge") {
        "MIT".into()
    } else if lower.contains("apache license") && lower.contains("version 2.0") {
        "Apache-2.0".into()
    } else if lower.contains("gnu general public license") {
        if lower.contains("version 3") { "GPL-3.0".into() }
        else { "GPL-2.0".into() }
    } else if lower.contains("bsd") {
        "BSD".into()
    } else if lower.contains("mozilla public license") {
        "MPL-2.0".into()
    } else {
        "Unknown".into()
    }
}

fn is_compatible(license_type: &str) -> bool {
    matches!(license_type, "MIT" | "Apache-2.0" | "BSD" | "MPL-2.0")
}
