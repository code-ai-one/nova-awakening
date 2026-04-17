#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 性能分析数据读取
/// 读取 Linux perf/pprof 格式的 profile 数据
/// 应用：PGO（Profile-Guided Optimization）反馈给编译器

use std::collections::HashMap;
use std::path::Path;

/// 函数热度记录
#[derive(Debug, Clone)]
pub struct FuncProfile {
    pub name:        String,
    pub samples:     u64,      // 采样次数（正比于执行次数）
    pub self_samples: u64,     // 不含调用子函数的采样
    pub call_sites:  HashMap<String, u64>,  // 调用的函数 → 采样数
}

impl FuncProfile {
    pub fn new(name: impl Into<String>) -> Self {
        FuncProfile { name: name.into(), samples: 0, self_samples: 0, call_sites: HashMap::new() }
    }
    pub fn add_sample(&mut self) { self.samples += 1; self.self_samples += 1; }
    pub fn add_call(&mut self, callee: impl Into<String>) {
        *self.call_sites.entry(callee.into()).or_default() += 1;
        self.samples += 1;
    }
    /// 热度得分（用于和 Nova 热度分析对接）
    pub fn heat_score(&self) -> u64 { self.samples }
    /// 最频繁被调用的函数（内联候选）
    pub fn hottest_callee(&self) -> Option<(&str, u64)> {
        self.call_sites.iter().max_by_key(|(_, &c)| c).map(|(n, &c)| (n.as_str(), c))
    }
}

/// 循环热度记录
#[derive(Debug, Clone)]
pub struct LoopProfile {
    pub func_name:    String,
    pub loop_header:  u64,    // 循环头基本块 ID
    pub iterations:   u64,   // 总迭代次数
    pub avg_iters:    f64,   // 平均每次调用的迭代次数
    pub vectorizable: bool,  // 是否在profile中表现为可向量化
}

/// 分支预测历史
#[derive(Debug, Clone)]
pub struct BranchProfile {
    pub pc:       u64,     // 指令地址
    pub taken:    u64,     // 跳转次数
    pub not_taken: u64,   // 不跳转次数
}

impl BranchProfile {
    pub fn taken_rate(&self) -> f64 {
        let total = self.taken + self.not_taken;
        if total == 0 { return 0.5; }
        self.taken as f64 / total as f64
    }
    pub fn is_biased(&self) -> bool {
        let rate = self.taken_rate();
        rate < 0.1 || rate > 0.9  // 超过90%倾向的分支
    }
    pub fn predicted_direction(&self) -> bool { self.taken > self.not_taken }
}

/// Profile 数据库
pub struct ProfileDB {
    pub funcs:    HashMap<String, FuncProfile>,
    pub loops:    Vec<LoopProfile>,
    pub branches: HashMap<u64, BranchProfile>,
    pub total_samples: u64,
}

impl ProfileDB {
    pub fn new() -> Self {
        ProfileDB { funcs: HashMap::new(), loops: vec![], branches: HashMap::new(), total_samples: 0 }
    }

    /// 记录函数采样
    pub fn record_func(&mut self, name: &str) {
        self.funcs.entry(name.to_string()).or_insert_with(|| FuncProfile::new(name)).add_sample();
        self.total_samples += 1;
    }

    /// 记录函数调用关系
    pub fn record_call(&mut self, caller: &str, callee: &str) {
        self.funcs.entry(caller.to_string()).or_insert_with(|| FuncProfile::new(caller)).add_call(callee);
    }

    /// 记录分支
    pub fn record_branch(&mut self, pc: u64, taken: bool) {
        let entry = self.branches.entry(pc).or_insert(BranchProfile { pc, taken: 0, not_taken: 0 });
        if taken { entry.taken += 1; } else { entry.not_taken += 1; }
    }

    /// 获取函数热度（归一化到0-100）
    pub fn func_heat_normalized(&self, name: &str) -> u32 {
        if self.total_samples == 0 { return 0; }
        let samples = self.funcs.get(name).map(|f| f.samples).unwrap_or(0);
        (samples * 100 / self.total_samples) as u32
    }

    /// Top N 热点函数
    pub fn top_funcs(&self, n: usize) -> Vec<(&FuncProfile, u32)> {
        let mut funcs: Vec<_> = self.funcs.values().collect();
        funcs.sort_by_key(|f| std::cmp::Reverse(f.samples));
        funcs.into_iter().take(n)
            .map(|f| (f, self.func_heat_normalized(&f.name)))
            .collect()
    }

    /// 从简单的文本格式读取（每行: 函数名 采样数）
    pub fn load_simple_text(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取profile失败: {}", e))?;
        let mut db = ProfileDB::new();
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0];
                let samples: u64 = parts[1].parse().unwrap_or(0);
                let entry = db.funcs.entry(name.to_string()).or_insert_with(|| FuncProfile::new(name));
                entry.samples += samples;
                entry.self_samples += samples;
                db.total_samples += samples;
            }
        }
        Ok(db)
    }

    /// 检查是否有有效的 profile 数据
    pub fn is_valid(&self) -> bool { self.total_samples >= 100 }

    /// 生成 PGO 注解（供 Nova 编译器使用）
    pub fn generate_annotations(&self) -> Vec<PgoAnnotation> {
        let mut annotations = vec![];
        for (name, func) in &self.funcs {
            let heat = self.func_heat_normalized(name);
            annotations.push(PgoAnnotation::FuncHeat { name: name.clone(), heat });
            if let Some((callee, count)) = func.hottest_callee() {
                if count * 10 >= func.samples {  // 占10%以上才建议内联
                    annotations.push(PgoAnnotation::InlineHint { caller: name.clone(), callee: callee.to_string() });
                }
            }
        }
        for (pc, branch) in &self.branches {
            if branch.is_biased() {
                annotations.push(PgoAnnotation::BranchBias { pc: *pc, likely_taken: branch.predicted_direction(), confidence: (branch.taken_rate() * 100.0) as u32 });
            }
        }
        annotations
    }

    pub fn stats(&self) -> String {
        format!("Profile: {}个函数 {}次采样 {}条分支记录",
            self.funcs.len(), self.total_samples, self.branches.len())
    }
}

impl Default for ProfileDB { fn default() -> Self { Self::new() } }

/// PGO 注解（传递给编译器的优化提示）
#[derive(Debug, Clone)]
pub enum PgoAnnotation {
    FuncHeat { name: String, heat: u32 },             // 函数热度 (0-100)
    InlineHint { caller: String, callee: String },     // 建议内联
    BranchBias { pc: u64, likely_taken: bool, confidence: u32 },  // 分支偏向
    LoopUnroll { func: String, header: u64, factor: u32 },  // 建议循环展开
    Vectorize { func: String, header: u64 },           // 建议向量化
}
