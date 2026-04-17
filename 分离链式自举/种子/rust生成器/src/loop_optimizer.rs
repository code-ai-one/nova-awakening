#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 循环优化助手
/// 支持：循环展开 / 循环融合 / 循环分裂 / 步幅优化
/// 配合 liveness_bits.rs 和 ssa_builder.rs 使用

use std::collections::{HashMap, HashSet};

/// 循环描述（从 Nova 分析层传入）
#[derive(Debug, Clone)]
pub struct LoopInfo {
    pub id:          u32,
    pub header:      u32,    // 循环头基本块 ID
    pub body:        Vec<u32>,  // 循环体所有基本块
    pub back_edges:  Vec<(u32, u32)>,  // (后继, 头部) 的回边
    pub depth:       u32,    // 循环嵌套深度
    pub trip_count:  Option<u64>,  // 已知迭代次数（None = 未知）
    pub is_innermost: bool,
    pub exit_blocks: Vec<u32>,   // 循环出口块
}

impl LoopInfo {
    pub fn size(&self) -> usize { self.body.len() }
    pub fn is_countable(&self) -> bool { self.trip_count.is_some() }

    /// 是否适合展开
    pub fn should_unroll(&self, max_body_size: usize, min_trip: u64) -> bool {
        if !self.is_innermost { return false; }
        if self.size() > max_body_size { return false; }
        if let Some(tc) = self.trip_count {
            return tc >= min_trip && tc <= 256;
        }
        false
    }

    /// 展开因子（展开多少次）
    pub fn unroll_factor(&self) -> u32 {
        if let Some(tc) = self.trip_count {
            if tc <= 4 { return tc as u32; }  // 完全展开
            if tc % 8 == 0 { return 8; }
            if tc % 4 == 0 { return 4; }
            if tc % 2 == 0 { return 2; }
        }
        // 未知迭代次数：部分展开
        if self.size() <= 5 { 4 } else { 2 }
    }
}

/// 循环优化决策
#[derive(Debug, Clone)]
pub struct LoopOptDecision {
    pub loop_id:   u32,
    pub action:    LoopAction,
    pub factor:    u32,   // 展开/融合因子
    pub expected_speedup: f64,  // 预期加速比
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoopAction {
    FullUnroll,    // 完全展开（消除循环开销）
    PartialUnroll, // 部分展开
    Vectorize,     // 向量化（转换为 SIMD）
    Fuse,          // 循环融合（与相邻循环合并）
    Split,         // 循环分裂（改善局部性）
    HoistInvariants,  // 提取循环不变量（已在 LICM pass 处理）
    Distribute,    // 循环分发（改善并行性）
    NoOp,          // 不优化
}

/// 循环优化器
pub struct LoopOptimizer {
    opt_level:   u8,
    pub decisions: Vec<LoopOptDecision>,
    pub stats:   LoopOptStats,
}

impl LoopOptimizer {
    pub fn new(opt_level: u8) -> Self {
        LoopOptimizer { opt_level, decisions: vec![], stats: LoopOptStats::default() }
    }

    /// 对一组循环做优化决策
    pub fn analyze_loops(&mut self, loops: &[LoopInfo]) {
        // 从最内层循环开始分析
        let mut sorted: Vec<_> = loops.iter().collect();
        sorted.sort_by_key(|l| std::cmp::Reverse(l.depth));

        for lp in &sorted {
            let decision = self.decide_for_loop(lp);
            self.decisions.push(decision);
        }
    }

    fn decide_for_loop(&self, lp: &LoopInfo) -> LoopOptDecision {
        // O0：不优化
        if self.opt_level == 0 {
            return LoopOptDecision { loop_id: lp.id, action: LoopAction::NoOp, factor: 1, expected_speedup: 1.0 };
        }

        // 完全展开：已知小迭代次数
        if let Some(tc) = lp.trip_count {
            if tc <= 8 && lp.is_innermost && lp.size() <= 10 {
                return LoopOptDecision {
                    loop_id: lp.id, action: LoopAction::FullUnroll,
                    factor: tc as u32, expected_speedup: 1.2 + tc as f64 * 0.02,
                };
            }
        }

        // O2+：向量化检查
        if self.opt_level >= 2 && lp.is_innermost {
            // 简化：如果循环体很小且可计数，尝试向量化
            if lp.size() <= 5 && lp.is_countable() {
                return LoopOptDecision {
                    loop_id: lp.id, action: LoopAction::Vectorize,
                    factor: 4, expected_speedup: 3.5,
                };
            }
        }

        // O1+：部分展开
        if self.opt_level >= 1 && lp.is_innermost {
            let factor = lp.unroll_factor();
            if factor >= 2 {
                return LoopOptDecision {
                    loop_id: lp.id, action: LoopAction::PartialUnroll,
                    factor, expected_speedup: 1.0 + factor as f64 * 0.05,
                };
            }
        }

        LoopOptDecision { loop_id: lp.id, action: LoopAction::NoOp, factor: 1, expected_speedup: 1.0 }
    }

    /// 对循环融合机会进行分析
    /// 相邻的两个循环，如果遍历相同的范围且没有依赖，可以融合
    pub fn find_fusion_candidates<'a>(&self, loops: &'a [LoopInfo]) -> Vec<(&'a LoopInfo, &'a LoopInfo)> {
        let mut candidates = vec![];
        for i in 0..loops.len().saturating_sub(1) {
            let a = &loops[i];
            let b = &loops[i + 1];
            // 简化的融合条件：都是最内层循环、相同迭代次数
            if a.is_innermost && b.is_innermost
                && a.trip_count == b.trip_count
                && a.trip_count.is_some()
                && a.depth == b.depth {
                candidates.push((a, b));
            }
        }
        candidates
    }

    /// 生成优化报告
    pub fn report(&self) -> String {
        let mut by_action: HashMap<String, usize> = HashMap::new();
        let mut total_speedup = 1.0f64;
        for d in &self.decisions {
            *by_action.entry(format!("{:?}", d.action)).or_default() += 1;
            total_speedup *= d.expected_speedup;
        }
        let summary: Vec<_> = by_action.iter()
            .filter(|(_, &c)| c > 0)
            .map(|(a, c)| format!("{}×{}", a, c)).collect();
        format!("循环优化: {}个循环 [{}] 预期加速{:.2}x", self.decisions.len(), summary.join(" "), total_speedup)
    }

    /// 记录执行统计
    pub fn record_applied(&mut self, loop_id: u32) {
        if let Some(d) = self.decisions.iter().find(|d| d.loop_id == loop_id) {
            match d.action {
                LoopAction::FullUnroll    => self.stats.fully_unrolled += 1,
                LoopAction::PartialUnroll => self.stats.partially_unrolled += 1,
                LoopAction::Vectorize     => self.stats.vectorized += 1,
                LoopAction::Fuse          => self.stats.fused += 1,
                _ => {}
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct LoopOptStats {
    pub fully_unrolled:    usize,
    pub partially_unrolled: usize,
    pub vectorized:        usize,
    pub fused:             usize,
}
impl LoopOptStats {
    pub fn format(&self) -> String {
        format!("循环统计: {}完全展开 {}部分展开 {}向量化 {}融合",
            self.fully_unrolled, self.partially_unrolled, self.vectorized, self.fused)
    }
    pub fn total_optimized(&self) -> usize {
        self.fully_unrolled + self.partially_unrolled + self.vectorized + self.fused
    }
}

/// 快速估算循环的向量化收益（基于循环体大小和迭代次数）
pub fn estimate_vectorization_speedup(lp: &LoopInfo, vector_width: u32) -> f64 {
    if !lp.is_innermost { return 1.0; }
    if let Some(tc) = lp.trip_count {
        let vector_iters = tc / vector_width as u64;
        let remainder = tc % vector_width as u64;
        if vector_iters == 0 { return 1.0; }
        let speedup = tc as f64 / (vector_iters as f64 + remainder as f64 * 0.8);
        return speedup.min(vector_width as f64 * 0.85);  // 实际加速通常低于理论值
    }
    (vector_width as f64 * 0.7).min(4.0)
}
