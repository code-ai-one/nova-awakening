#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 诊断输出模块
/// 提供带颜色/源码位置/修复建议的丰富错误报告
/// 参考 Rust 编译器的高质量诊断体验

use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiagLevel {
    Error,
    Warning,
    Note,
    Help,
}

impl DiagLevel {
    pub fn prefix(self) -> &'static str {
        match self {
            DiagLevel::Error   => "\x1b[31merror\x1b[0m",
            DiagLevel::Warning => "\x1b[33mwarning\x1b[0m",
            DiagLevel::Note    => "\x1b[36mnote\x1b[0m",
            DiagLevel::Help    => "\x1b[32mhelp\x1b[0m",
        }
    }
    pub fn prefix_plain(self) -> &'static str {
        match self {
            DiagLevel::Error   => "错误",
            DiagLevel::Warning => "警告",
            DiagLevel::Note    => "提示",
            DiagLevel::Help    => "建议",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceSpan {
    pub file: String,
    pub line: usize,
    pub col:  usize,
    pub len:  usize,
}

impl SourceSpan {
    pub fn new(file: impl Into<String>, line: usize, col: usize, len: usize) -> Self {
        SourceSpan { file: file.into(), line, col, len }
    }
    pub fn unknown() -> Self { SourceSpan { file: "<未知>".into(), line: 0, col: 0, len: 0 } }
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub span:    SourceSpan,
    pub message: String,
    pub primary: bool,
}

#[derive(Debug, Clone)]
pub struct FixSuggestion {
    pub span:        Option<SourceSpan>,
    pub replacement: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level:       DiagLevel,
    pub code:        u32,
    pub message:     String,
    pub annotations: Vec<Annotation>,
    pub fixes:       Vec<FixSuggestion>,
    pub notes:       Vec<String>,
}

impl Diagnostic {
    pub fn error(code: u32, msg: impl Into<String>) -> Self {
        Diagnostic { level: DiagLevel::Error, code, message: msg.into(),
                     annotations: vec![], fixes: vec![], notes: vec![] }
    }
    pub fn warning(code: u32, msg: impl Into<String>) -> Self {
        Diagnostic { level: DiagLevel::Warning, code, message: msg.into(),
                     annotations: vec![], fixes: vec![], notes: vec![] }
    }

    pub fn with_span(mut self, span: SourceSpan, label: impl Into<String>, primary: bool) -> Self {
        self.annotations.push(Annotation { span, message: label.into(), primary });
        self
    }
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
    pub fn with_fix(mut self, desc: impl Into<String>, replacement: impl Into<String>) -> Self {
        self.fixes.push(FixSuggestion { span: None, replacement: replacement.into(),
                                         description: desc.into() });
        self
    }
    pub fn is_error(&self) -> bool { self.level == DiagLevel::Error }
}

/// 诊断收集器
#[derive(Default)]
pub struct DiagCollector {
    pub diags:   Vec<Diagnostic>,
    pub errors:  usize,
    pub warnings: usize,
}

impl DiagCollector {
    pub fn new() -> Self { DiagCollector::default() }

    pub fn add(&mut self, d: Diagnostic) {
        if d.is_error() { self.errors += 1; } else { self.warnings += 1; }
        self.diags.push(d);
    }

    pub fn has_errors(&self) -> bool { self.errors > 0 }

    /// 格式化所有诊断为文本（无颜色，适合文件输出）
    pub fn format_plain(&self) -> String {
        let mut out = String::new();
        for d in &self.diags {
            let _ = writeln!(out, "[{}E{:04}] {}", d.level.prefix_plain(), d.code, d.message);
            for ann in &d.annotations {
                let _ = writeln!(out, "  --> {}:{}:{}", ann.span.file, ann.span.line, ann.span.col);
                if !ann.message.is_empty() {
                    let _ = writeln!(out, "  | {}", ann.message);
                }
            }
            for note in &d.notes {
                let _ = writeln!(out, "  = 说明: {}", note);
            }
            for fix in &d.fixes {
                let _ = writeln!(out, "  = 建议: {}", fix.description);
            }
        }
        if self.errors > 0 || self.warnings > 0 {
            let _ = writeln!(out, "\n编译结果: {} 个错误, {} 个警告", self.errors, self.warnings);
        }
        out
    }

    /// 格式化为带ANSI颜色的终端输出
    pub fn format_colored(&self) -> String {
        let mut out = String::new();
        for d in &self.diags {
            let _ = writeln!(out, "{}\x1b[1m[E{:04}]\x1b[0m: \x1b[1m{}\x1b[0m",
                d.level.prefix(), d.code, d.message);
            for ann in &d.annotations {
                let arrow = if ann.primary { "\x1b[34m-->\x1b[0m" } else { "\x1b[34m  =\x1b[0m" };
                let _ = writeln!(out, "  {} \x1b[4m{}:{}:{}\x1b[0m", arrow,
                    ann.span.file, ann.span.line, ann.span.col);
                if !ann.message.is_empty() {
                    let _ = writeln!(out, "  \x1b[34m|\x1b[0m \x1b[{}m{}\x1b[0m",
                        if ann.primary { "31" } else { "33" }, ann.message);
                }
            }
            for note in &d.notes {
                let _ = writeln!(out, "  \x1b[36m= 说明:\x1b[0m {}", note);
            }
            for fix in &d.fixes {
                let _ = writeln!(out, "  \x1b[32m= 建议:\x1b[0m {} `{}`",
                    fix.description, fix.replacement);
            }
            out.push('\n');
        }
        out
    }

    pub fn print_summary(&self) {
        if self.errors > 0 {
            eprintln!("\x1b[31m编译失败\x1b[0m: {} 个错误, {} 个警告", self.errors, self.warnings);
        } else if self.warnings > 0 {
            eprintln!("\x1b[33m编译成功（有警告）\x1b[0m: {} 个警告", self.warnings);
        }
    }
}

/// 常用诊断工厂函数
pub fn diag_undefined_symbol(file: &str, line: usize, col: usize, name: &str) -> Diagnostic {
    Diagnostic::error(3000, format!("未定义的符号: `{}`", name))
        .with_span(SourceSpan::new(file, line, col, name.len()), "在此使用了未定义符号", true)
        .with_note(format!("确认 `{}` 是否已定义，或是否存在拼写错误", name))
}

pub fn diag_type_mismatch(file: &str, line: usize, expected: &str, got: &str) -> Diagnostic {
    Diagnostic::error(2000, format!("类型不匹配: 期望 `{}` 但得到 `{}`", expected, got))
        .with_span(SourceSpan::new(file, line, 0, 0), format!("此处类型为 `{}`", got), true)
}

pub fn diag_immutable_assign(file: &str, line: usize, var: &str) -> Diagnostic {
    Diagnostic::error(4000, format!("不可变变量 `{}` 不能被赋值", var))
        .with_span(SourceSpan::new(file, line, 0, var.len()), "尝试修改不可变变量", true)
        .with_fix(format!("将声明改为 `定义 可变 {} = ...`", var), format!("定义 可变 {}", var))
}

pub fn diag_unused_variable(file: &str, line: usize, var: &str) -> Diagnostic {
    Diagnostic::warning(3100, format!("变量 `{}` 未使用", var))
        .with_span(SourceSpan::new(file, line, 0, var.len()), "此变量被声明但从未使用", true)
        .with_fix(format!("如果有意忽略，将变量名改为 `_{}`", var), format!("_{}", var))
}
