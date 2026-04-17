#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova IR Pass 管理器
/// 编排所有优化 Pass 的执行顺序，支持依赖关系和重复运行
/// 分三类：Analysis Pass / Transform Pass / Verification Pass

use std::collections::{HashMap, HashSet};

/// Pass 类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PassKind {
    Analysis,    // 只读，计算分析结果（活跃性/支配树等）
    Transform,   // 变换 IR（常量折叠/内联等）
    Verify,      // 验证 IR 的不变量（调试用）
}

/// Pass 状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PassStatus { Pending, Running, Done, Skipped, Failed }

/// Pass 执行记录
#[derive(Debug, Clone)]
pub struct PassRecord {
    pub name:        String,
    pub kind:        PassKind,
    pub status:      PassStatus,
    pub iterations:  u32,
    pub changes:     u32,  // Transform Pass: 做了多少处变换
    pub time_us:     u64,  // 执行时间（微秒）
}

impl PassRecord {
    pub fn new(name: impl Into<String>, kind: PassKind) -> Self {
        PassRecord { name: name.into(), kind, status: PassStatus::Pending, iterations: 0, changes: 0, time_us: 0 }
    }
    pub fn format(&self) -> String {
        let status = match self.status {
            PassStatus::Done    => "✓",
            PassStatus::Skipped => "⊘",
            PassStatus::Failed  => "✗",
            PassStatus::Running => "▶",
            PassStatus::Pending => "○",
        };
        format!("[{}] {:30} {}轮 {}变 {}μs", status, self.name, self.iterations, self.changes, self.time_us)
    }
}

/// Pass 管线定义
pub struct PassPipeline {
    passes:      Vec<(String, PassKind, Vec<String>)>,  // (名称, 类型, 依赖列表)
    order:       Vec<String>,  // 拓扑排序后的执行顺序
    records:     HashMap<String, PassRecord>,
    opt_level:   u8,
}

impl PassPipeline {
    pub fn new(opt_level: u8) -> Self {
        PassPipeline { passes: vec![], order: vec![], records: HashMap::new(), opt_level }
    }

    pub fn add(&mut self, name: impl Into<String>, kind: PassKind, deps: Vec<&str>) {
        let n = name.into();
        let d: Vec<String> = deps.into_iter().map(|s| s.to_string()).collect();
        self.records.insert(n.clone(), PassRecord::new(n.clone(), kind));
        self.passes.push((n, kind, d));
    }

    /// 按优化级别构建标准管线
    pub fn standard_pipeline(opt_level: u8) -> Self {
        let mut p = PassPipeline::new(opt_level);
        // ── 分析 Pass ──
        p.add("活跃变量分析",      PassKind::Analysis,   vec![]);
        p.add("到达定义分析",      PassKind::Analysis,   vec![]);
        p.add("支配树构建",        PassKind::Analysis,   vec![]);
        p.add("循环识别",          PassKind::Analysis,   vec!["支配树构建"]);
        p.add("别名分析",          PassKind::Analysis,   vec![]);
        p.add("调用图构建",        PassKind::Analysis,   vec![]);
        // ── O1+ 变换 Pass ──
        if opt_level >= 1 {
            p.add("常量折叠",          PassKind::Transform,  vec![]);
            p.add("死代码消除",        PassKind::Transform,  vec!["活跃变量分析"]);
            p.add("窥孔优化",          PassKind::Transform,  vec![]);
        }
        // ── O2+ 变换 Pass ──
        if opt_level >= 2 {
            p.add("常量传播",          PassKind::Transform,  vec!["到达定义分析"]);
            p.add("循环不变量提升",    PassKind::Transform,  vec!["循环识别", "别名分析"]);
            p.add("公共子表达式消除",  PassKind::Transform,  vec!["支配树构建"]);
            p.add("函数内联",          PassKind::Transform,  vec!["调用图构建"]);
            p.add("尾调用优化",        PassKind::Transform,  vec![]);
        }
        // ── O3 变换 Pass ──
        if opt_level >= 3 {
            p.add("循环展开",          PassKind::Transform,  vec!["循环识别"]);
            p.add("向量化",            PassKind::Transform,  vec!["循环识别", "别名分析"]);
            p.add("过程间常量传播",    PassKind::Transform,  vec!["调用图构建", "常量传播"]);
            p.add("热度引导内联",      PassKind::Transform,  vec!["调用图构建"]);
        }
        // ── 后端 Pass ──
        p.add("指令调度",          PassKind::Transform,  vec![]);
        p.add("寄存器分配",        PassKind::Transform,  vec!["活跃变量分析"]);
        // ── 验证 Pass（调试模式）──
        if opt_level == 0 {
            p.add("SSA验证",           PassKind::Verify,     vec![]);
            p.add("类型检查",          PassKind::Verify,     vec![]);
        }
        let _ = p.topo_sort();
        p
    }

    /// 拓扑排序（Kahn算法）
    pub fn topo_sort(&mut self) -> Result<(), String> {
        let mut in_deg: HashMap<String, usize> = self.passes.iter().map(|(n, _, _)| (n.clone(), 0)).collect();
        for (name, _, deps) in &self.passes {
            for dep in deps {
                if in_deg.contains_key(dep) {
                    *in_deg.get_mut(name).unwrap() += 1;
                }
            }
        }
        let mut queue: Vec<String> = in_deg.iter().filter(|(_, &d)| d == 0).map(|(n, _)| n.clone()).collect();
        let mut order = vec![];
        // 反向邻接表
        let mut rev_adj: HashMap<String, Vec<String>> = HashMap::new();
        for (name, _, deps) in &self.passes {
            for dep in deps { rev_adj.entry(dep.clone()).or_default().push(name.clone()); }
        }
        while !queue.is_empty() {
            let n = queue.remove(0);
            order.push(n.clone());
            if let Some(deps) = rev_adj.get(&n) {
                for dep in deps {
                    let d = in_deg.get_mut(dep).unwrap();
                    *d -= 1;
                    if *d == 0 { queue.push(dep.clone()); }
                }
            }
        }
        if order.len() != self.passes.len() {
            return Err("Pass 管线存在循环依赖".to_string());
        }
        self.order = order;
        Ok(())
    }

    /// 标记 Pass 开始
    pub fn begin_pass(&mut self, name: &str) {
        if let Some(r) = self.records.get_mut(name) {
            r.status = PassStatus::Running;
            r.iterations += 1;
        }
    }

    /// 标记 Pass 完成
    pub fn end_pass(&mut self, name: &str, changes: u32, time_us: u64) {
        if let Some(r) = self.records.get_mut(name) {
            r.status = PassStatus::Done;
            r.changes += changes;
            r.time_us += time_us;
        }
    }

    pub fn skip_pass(&mut self, name: &str) {
        if let Some(r) = self.records.get_mut(name) { r.status = PassStatus::Skipped; }
    }
    pub fn fail_pass(&mut self, name: &str) {
        if let Some(r) = self.records.get_mut(name) { r.status = PassStatus::Failed; }
    }

    /// 获取执行顺序
    pub fn execution_order(&self) -> &[String] { &self.order }

    /// 生成执行报告
    pub fn report(&self) -> String {
        let mut out = format!("=== Pass 管线报告 (O{}) ===\n", self.opt_level);
        for name in &self.order {
            if let Some(rec) = self.records.get(name) {
                out += &format!("  {}\n", rec.format());
            }
        }
        let total_changes: u32 = self.records.values().map(|r| r.changes).sum();
        let total_time: u64 = self.records.values().map(|r| r.time_us).sum();
        let failed = self.records.values().filter(|r| r.status == PassStatus::Failed).count();
        out += &format!("  合计: {}次变换 {:.1}ms {}", total_changes, total_time as f64 / 1000.0,
            if failed > 0 { format!("⚠ {}个Pass失败", failed) } else { "✓ 全部通过".into() });
        out
    }

    pub fn total_transforms(&self) -> u32 {
        self.records.values().filter(|r| r.kind == PassKind::Transform).map(|r| r.changes).sum()
    }
}
