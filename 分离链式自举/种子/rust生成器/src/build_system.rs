#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 构建系统集成
/// 依赖图分析 / 最小增量重建 / 并行编译调度

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// 构建目标
#[derive(Debug, Clone)]
pub struct BuildTarget {
    pub name:     String,
    pub source:   PathBuf,
    pub output:   PathBuf,
    pub deps:     Vec<String>,    // 依赖的其他目标名
    pub phony:    bool,           // 是否是伪目标（不产生文件）
}

impl BuildTarget {
    pub fn new(name: impl Into<String>, source: impl Into<PathBuf>, output: impl Into<PathBuf>) -> Self {
        BuildTarget { name: name.into(), source: source.into(),
                      output: output.into(), deps: vec![], phony: false }
    }
    pub fn with_deps(mut self, deps: Vec<impl Into<String>>) -> Self {
        self.deps = deps.into_iter().map(|d| d.into()).collect(); self
    }
}

/// 构建状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildStatus {
    Clean,       // 最新，不需要重建
    Dirty,       // 需要重建
    Building,    // 正在构建
    Done,        // 构建完成
    Failed,      // 构建失败
}

/// 构建图
pub struct BuildGraph {
    targets:  HashMap<String, BuildTarget>,
    status:   HashMap<String, BuildStatus>,
    order:    Vec<String>,   // 拓扑排序后的构建顺序
}

impl BuildGraph {
    pub fn new() -> Self {
        BuildGraph { targets: HashMap::new(), status: HashMap::new(), order: vec![] }
    }

    pub fn add_target(&mut self, target: BuildTarget) {
        let name = target.name.clone();
        self.targets.insert(name.clone(), target);
        self.status.insert(name, BuildStatus::Dirty);
    }

    /// 计算哪些目标需要重建（基于文件修改时间）
    pub fn compute_dirty(&mut self) {
        for (name, target) in &self.targets {
            let dirty = if target.phony {
                true
            } else if !target.output.exists() {
                true
            } else {
                let out_mtime = mtime(&target.output);
                let src_mtime = mtime(&target.source);
                src_mtime > out_mtime || target.deps.iter().any(|dep| {
                    self.targets.get(dep)
                        .map(|dt| mtime(&dt.output) > out_mtime)
                        .unwrap_or(true)
                })
            };
            self.status.insert(name.clone(), if dirty { BuildStatus::Dirty } else { BuildStatus::Clean });
        }
    }

    /// 拓扑排序（Kahn算法）
    pub fn topo_sort(&mut self) -> Result<(), String> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for name in self.targets.keys() { in_degree.insert(name.clone(), 0); }
        for target in self.targets.values() {
            for dep in &target.deps {
                *in_degree.entry(dep.clone()).or_default() += 0;
                if self.targets.contains_key(dep) {
                    *in_degree.get_mut(&target.name).unwrap() += 1;
                }
            }
        }
        let mut queue: VecDeque<String> = in_degree.iter()
            .filter(|(_, &d)| d == 0).map(|(n, _)| n.clone()).collect();
        let mut order = vec![];
        while let Some(name) = queue.pop_front() {
            order.push(name.clone());
            // 找依赖该目标的所有目标
            for (tname, target) in &self.targets {
                if target.deps.contains(&name) {
                    let deg = in_degree.get_mut(tname).unwrap();
                    *deg -= 1;
                    if *deg == 0 { queue.push_back(tname.clone()); }
                }
            }
        }
        if order.len() != self.targets.len() {
            return Err("构建图存在循环依赖".to_string());
        }
        self.order = order;
        Ok(())
    }

    /// 获取需要重建的目标（按拓扑顺序）
    pub fn dirty_targets(&self) -> Vec<&BuildTarget> {
        self.order.iter()
            .filter(|n| self.status.get(*n) == Some(&BuildStatus::Dirty))
            .filter_map(|n| self.targets.get(n))
            .collect()
    }

    /// 标记目标构建完成
    pub fn mark_done(&mut self, name: &str) {
        self.status.insert(name.to_string(), BuildStatus::Done);
    }
    pub fn mark_failed(&mut self, name: &str) {
        self.status.insert(name.to_string(), BuildStatus::Failed);
    }

    pub fn all_done(&self) -> bool {
        self.targets.keys().all(|n| {
            matches!(self.status.get(n), Some(BuildStatus::Done) | Some(BuildStatus::Clean))
        })
    }

    pub fn has_failures(&self) -> bool {
        self.status.values().any(|s| *s == BuildStatus::Failed)
    }

    /// 生成 Makefile 格式（可用于 make 重建）
    pub fn to_makefile(&self) -> String {
        let mut out = String::from("# 由 Nova 构建系统自动生成\n\n");
        let all: Vec<_> = self.targets.values().filter(|t| !t.phony).map(|t| t.output.display().to_string()).collect();
        out += &format!("all: {}\n\n", all.join(" "));
        for target in self.targets.values() {
            let dep_files: Vec<_> = std::iter::once(target.source.display().to_string())
                .chain(target.deps.iter().filter_map(|d| self.targets.get(d)).map(|dt| dt.output.display().to_string()))
                .collect();
            out += &format!("{}: {}\n", target.output.display(), dep_files.join(" "));
            out += &format!("\t@echo \"构建: {}\"\n\n", target.name);
        }
        out
    }

    pub fn stats(&self) -> BuildStats {
        let clean = self.status.values().filter(|&&s| s == BuildStatus::Clean).count();
        let dirty = self.status.values().filter(|&&s| s == BuildStatus::Dirty).count();
        let done  = self.status.values().filter(|&&s| s == BuildStatus::Done).count();
        let failed = self.status.values().filter(|&&s| s == BuildStatus::Failed).count();
        BuildStats { total: self.targets.len(), clean, dirty, done, failed }
    }
}

impl Default for BuildGraph { fn default() -> Self { Self::new() } }

fn mtime(path: &Path) -> SystemTime {
    path.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH)
}

#[derive(Debug)]
pub struct BuildStats {
    pub total: usize, pub clean: usize, pub dirty: usize, pub done: usize, pub failed: usize,
}
impl BuildStats {
    pub fn format(&self) -> String {
        format!("构建: {}个目标 ({}最新 {}需重建 {}完成 {}失败)",
            self.total, self.clean, self.dirty, self.done, self.failed)
    }
}

/// 从 Nova manifest 构建构建图
pub fn graph_from_manifest(manifest_path: &Path, kernel_root: &Path, output_dir: &Path) -> BuildGraph {
    let mut graph = BuildGraph::new();
    if let Ok(content) = std::fs::read_to_string(manifest_path) {
        let mut prev: Option<String> = None;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            let src = kernel_root.join(line);
            let out_name = line.replace('/', "_").replace(".nova", ".o");
            let out = output_dir.join(&out_name);
            let mut target = BuildTarget::new(line, src, out);
            if let Some(p) = prev {
                target.deps.push(p);
            }
            prev = Some(line.to_string());
            graph.add_target(target);
        }
    }
    graph
}
