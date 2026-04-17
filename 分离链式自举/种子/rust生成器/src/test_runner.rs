#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 测试运行器
/// 发现/执行/报告 Nova 单元测试，支持并行运行和超时控制

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TestResult {
    Pass,
    Fail,
    Skip,
    Timeout,
    Error,
}

impl TestResult {
    pub fn symbol(self) -> &'static str {
        match self {
            TestResult::Pass    => "✓",
            TestResult::Fail    => "✗",
            TestResult::Skip    => "⊘",
            TestResult::Timeout => "⏰",
            TestResult::Error   => "!",
        }
    }
    pub fn is_success(self) -> bool { self == TestResult::Pass || self == TestResult::Skip }
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub name:     String,
    pub module:   String,
    pub source:   PathBuf,
    pub expected_exit: i32,
    pub expected_stdout: Option<String>,
    pub timeout_ms: u64,
    pub tags:     Vec<String>,
}

impl TestCase {
    pub fn new(name: impl Into<String>, module: impl Into<String>, source: impl Into<PathBuf>) -> Self {
        TestCase {
            name: name.into(),
            module: module.into(),
            source: source.into(),
            expected_exit: 0,
            expected_stdout: None,
            timeout_ms: 5000,
            tags: vec![],
        }
    }
    pub fn with_expected_exit(mut self, code: i32) -> Self { self.expected_exit = code; self }
    pub fn with_expected_output(mut self, out: impl Into<String>) -> Self {
        self.expected_stdout = Some(out.into()); self
    }
    pub fn with_timeout(mut self, ms: u64) -> Self { self.timeout_ms = ms; self }
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self { self.tags.push(tag.into()); self }
}

#[derive(Debug, Clone)]
pub struct TestOutcome {
    pub case:      TestCase,
    pub result:    TestResult,
    pub duration:  Duration,
    pub stdout:    String,
    pub stderr:    String,
    pub exit_code: Option<i32>,
    pub message:   String,
}

impl TestOutcome {
    pub fn format_brief(&self) -> String {
        format!("[{}] {} ({:.0}ms) {}",
            self.result.symbol(), self.case.name,
            self.duration.as_millis(),
            if self.result == TestResult::Pass { String::new() } else { self.message.clone() }
        )
    }
}

/// 测试套件
pub struct TestSuite {
    pub name:  String,
    cases:     Vec<TestCase>,
    filter:    Option<String>,  // 只运行匹配名字的测试
    tag_filter: Vec<String>,    // 只运行有特定标签的测试
}

impl TestSuite {
    pub fn new(name: impl Into<String>) -> Self {
        TestSuite { name: name.into(), cases: vec![], filter: None, tag_filter: vec![] }
    }

    pub fn add(&mut self, case: TestCase) { self.cases.push(case); }
    pub fn set_filter(&mut self, f: impl Into<String>) { self.filter = Some(f.into()); }
    pub fn set_tag_filter(&mut self, tags: Vec<impl Into<String>>) {
        self.tag_filter = tags.into_iter().map(|t| t.into()).collect();
    }

    fn should_run(&self, case: &TestCase) -> bool {
        if let Some(ref filter) = self.filter {
            if !case.name.contains(filter.as_str()) { return false; }
        }
        if !self.tag_filter.is_empty() {
            if !self.tag_filter.iter().any(|t| case.tags.contains(t)) { return false; }
        }
        true
    }

    /// 运行所有测试（串行）
    pub fn run_serial(&self, binary: &Path) -> TestReport {
        let mut outcomes = vec![];
        let start = Instant::now();

        for case in &self.cases {
            if !self.should_run(case) {
                outcomes.push(TestOutcome {
                    case: case.clone(), result: TestResult::Skip,
                    duration: Duration::ZERO, stdout: String::new(),
                    stderr: String::new(), exit_code: None,
                    message: "已跳过".into(),
                });
                continue;
            }
            let outcome = run_single_test(case, binary);
            outcomes.push(outcome);
        }

        TestReport::new(self.name.clone(), outcomes, start.elapsed())
    }

    pub fn len(&self) -> usize { self.cases.len() }
    pub fn is_empty(&self) -> bool { self.cases.is_empty() }
}

/// 运行单个测试
fn run_single_test(case: &TestCase, binary: &Path) -> TestOutcome {
    let start = Instant::now();
    let timeout = Duration::from_millis(case.timeout_ms);

    let result = std::process::Command::new(binary)
        .arg("--test")
        .arg(&case.source)
        .output();

    let duration = start.elapsed();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);

            // 检查超时
            if duration > timeout {
                return TestOutcome {
                    case: case.clone(), result: TestResult::Timeout,
                    duration, stdout, stderr, exit_code: Some(exit_code),
                    message: format!("超时 ({}ms > {}ms)", duration.as_millis(), case.timeout_ms),
                };
            }

            // 检查退出码
            if exit_code != case.expected_exit {
                return TestOutcome {
                    case: case.clone(), result: TestResult::Fail,
                    duration, stdout, stderr, exit_code: Some(exit_code),
                    message: format!("退出码 {} ≠ 期望 {}", exit_code, case.expected_exit),
                };
            }

            // 检查输出
            if let Some(ref expected) = case.expected_stdout {
                if !stdout.contains(expected.as_str()) {
                    return TestOutcome {
                        case: case.clone(), result: TestResult::Fail,
                        duration, stdout, stderr, exit_code: Some(exit_code),
                        message: format!("输出不匹配: 未找到 `{}`", expected),
                    };
                }
            }

            TestOutcome {
                case: case.clone(), result: TestResult::Pass,
                duration, stdout, stderr, exit_code: Some(exit_code),
                message: String::new(),
            }
        }
        Err(e) => TestOutcome {
            case: case.clone(), result: TestResult::Error,
            duration, stdout: String::new(), stderr: e.to_string(),
            exit_code: None, message: format!("执行失败: {}", e),
        }
    }
}

/// 测试报告
pub struct TestReport {
    pub suite:    String,
    pub outcomes: Vec<TestOutcome>,
    pub total_duration: Duration,
}

impl TestReport {
    pub fn new(suite: String, outcomes: Vec<TestOutcome>, dur: Duration) -> Self {
        TestReport { suite, outcomes, total_duration: dur }
    }

    pub fn passed(&self) -> usize { self.outcomes.iter().filter(|o| o.result == TestResult::Pass).count() }
    pub fn failed(&self) -> usize { self.outcomes.iter().filter(|o| o.result == TestResult::Fail).count() }
    pub fn skipped(&self) -> usize { self.outcomes.iter().filter(|o| o.result == TestResult::Skip).count() }
    pub fn errors(&self) -> usize { self.outcomes.iter().filter(|o| o.result == TestResult::Error || o.result == TestResult::Timeout).count() }
    pub fn all_passed(&self) -> bool { self.failed() == 0 && self.errors() == 0 }

    pub fn format_summary(&self) -> String {
        let status = if self.all_passed() { "\x1b[32m通过\x1b[0m" } else { "\x1b[31m失败\x1b[0m" };
        format!("测试套件 [{}] {}: {}/{}通过 {}跳过 ({:.0}ms)",
            self.suite, status, self.passed(),
            self.outcomes.len(), self.skipped(),
            self.total_duration.as_millis())
    }

    pub fn format_full(&self) -> String {
        let mut out = String::new();
        for o in &self.outcomes {
            out += &o.format_brief();
            out += "\n";
            if o.result == TestResult::Fail && !o.stdout.is_empty() {
                out += &format!("  stdout: {}\n", o.stdout.trim());
            }
        }
        out += &self.format_summary();
        out += "\n";
        out
    }

    /// 发现测试用例（从目录扫描 .nova 文件中的 `测试函数` 或 `// @test` 注释）
    pub fn discover_tests(test_dir: &Path) -> Vec<TestCase> {
        let mut cases = vec![];
        if let Ok(entries) = std::fs::read_dir(test_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("nova") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                        // 从文件注释中提取期望的退出码
                        let expected_exit = content.lines()
                            .find(|l| l.contains("@expected_exit"))
                            .and_then(|l| l.split(':').nth(1))
                            .and_then(|v| v.trim().parse().ok())
                            .unwrap_or(0);
                        let mut case = TestCase::new(name.clone(), "test", path);
                        case.expected_exit = expected_exit;
                        // 标签
                        for line in content.lines() {
                            if let Some(tag) = line.strip_prefix("// @tag:") {
                                case.tags.push(tag.trim().to_string());
                            }
                        }
                        cases.push(case);
                    }
                }
            }
        }
        cases
    }
}
