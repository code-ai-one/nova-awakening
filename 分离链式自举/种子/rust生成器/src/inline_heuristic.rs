#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 内联启发式决策
/// 决定是否内联一个函数调用，综合多个维度评分
/// 基于：函数体大小/调用频率/递归深度/参数特化机会

use std::collections::HashMap;

/// 内联决策
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InlineDecision {
    Always,   // 强制内联（小函数/hot path）
    Inline,   // 建议内联
    Maybe,    // 边界情况，看上下文
    NoInline, // 不内联（太大/递归/内联爆炸风险）
}

/// 函数的内联属性（编译期收集）
#[derive(Debug, Clone)]
pub struct FuncInlineInfo {
    pub name:          String,
    pub ir_size:       u32,    // IR 指令数（内联代价估算）
    pub call_count:    u64,    // 调用次数（来自 profile 或静态估算）
    pub is_recursive:  bool,   // 是否递归
    pub has_varargs:   bool,   // 是否变参
    pub param_count:   u32,
    pub const_params:  u32,    // 调用点上常量参数数量（特化机会）
    pub loop_depth:    u32,    // 所在循环深度（越深越值得内联）
    pub attribute:     InlineAttr,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InlineAttr {
    None,
    ForceInline,   // 用户标注 @内联
    NoInline,      // 用户标注 @不内联
}

/// 内联预算（防止代码体积爆炸）
#[derive(Debug, Clone)]
pub struct InlineBudget {
    pub max_ir_size:          u32,  // 被内联函数最大IR大小
    pub max_inline_depth:     u32,  // 最大内联深度
    pub max_total_expansion:  u32,  // 总膨胀系数（百分比，100=不膨胀）
    pub used_expansion:       u32,  // 已用膨胀量
}

impl InlineBudget {
    pub fn for_opt_level(opt: u8) -> Self {
        match opt {
            0 => InlineBudget { max_ir_size: 5,   max_inline_depth: 1, max_total_expansion: 110, used_expansion: 0 },
            1 => InlineBudget { max_ir_size: 30,  max_inline_depth: 2, max_total_expansion: 150, used_expansion: 0 },
            2 => InlineBudget { max_ir_size: 100, max_inline_depth: 4, max_total_expansion: 300, used_expansion: 0 },
            _ => InlineBudget { max_ir_size: 300, max_inline_depth: 8, max_total_expansion: 500, used_expansion: 0 },
        }
    }

    pub fn can_afford(&self, size: u32) -> bool {
        self.used_expansion + size <= self.max_total_expansion
    }

    pub fn spend(&mut self, size: u32) {
        self.used_expansion += size;
    }

    pub fn remaining(&self) -> u32 {
        self.max_total_expansion.saturating_sub(self.used_expansion)
    }
}

/// 内联评分器
pub struct InlineHeuristic {
    pub budget: InlineBudget,
    pub depth:  u32,           // 当前内联嵌套深度
    pub stats:  InlineStats,
}

impl InlineHeuristic {
    pub fn new(opt_level: u8) -> Self {
        InlineHeuristic { budget: InlineBudget::for_opt_level(opt_level), depth: 0, stats: InlineStats::default() }
    }

    /// 核心决策函数
    pub fn decide(&mut self, info: &FuncInlineInfo) -> InlineDecision {
        // 用户强制内联
        if info.attribute == InlineAttr::ForceInline { return InlineDecision::Always; }
        // 用户禁止内联
        if info.attribute == InlineAttr::NoInline   { return InlineDecision::NoInline; }
        // 递归函数不内联
        if info.is_recursive { return InlineDecision::NoInline; }
        // 变参函数不内联
        if info.has_varargs  { return InlineDecision::NoInline; }
        // 超出嵌套深度
        if self.depth >= self.budget.max_inline_depth { return InlineDecision::NoInline; }
        // 预算用完
        if !self.budget.can_afford(info.ir_size) { return InlineDecision::NoInline; }

        // 计算得分
        let score = self.score(info);

        if score >= 80 { InlineDecision::Always }
        else if score >= 50 { InlineDecision::Inline }
        else if score >= 30 { InlineDecision::Maybe }
        else { InlineDecision::NoInline }
    }

    /// 综合评分（0-100）
    fn score(&self, info: &FuncInlineInfo) -> u32 {
        let mut score = 50u32;

        // 函数体大小（小函数加分）
        let size_score: i32 = if info.ir_size <= 5 { 40 }
            else if info.ir_size <= 20 { 20 }
            else if info.ir_size <= 50 { 0 }
            else if info.ir_size <= 100 { -20 }
            else { -50 };
        score = score.saturating_add(size_score.unsigned_abs()).min(100).saturating_sub((-size_score).max(0) as u32);

        // 调用频率（热点加分）
        if info.call_count >= 1000 { score = score.saturating_add(20); }
        else if info.call_count >= 100 { score = score.saturating_add(10); }
        else if info.call_count <= 1 { score = score.saturating_sub(10); }

        // 常量参数特化机会
        if info.const_params > 0 {
            score = score.saturating_add(info.const_params * 10);
        }

        // 在循环内（热点）
        if info.loop_depth > 0 {
            score = score.saturating_add(info.loop_depth.min(3) * 10);
        }

        // 无参数函数：完全透明内联
        if info.param_count == 0 { score = score.saturating_add(20); }

        score.min(100)
    }

    /// 记录内联决策结果
    pub fn record(&mut self, decision: InlineDecision, info: &FuncInlineInfo) {
        match decision {
            InlineDecision::Always | InlineDecision::Inline => {
                self.budget.spend(info.ir_size);
                self.stats.inlined += 1;
                self.stats.total_ir_added += info.ir_size;
            }
            InlineDecision::Maybe => { self.stats.maybe += 1; }
            InlineDecision::NoInline => { self.stats.not_inlined += 1; }
        }
    }

    /// 进入被内联函数（增加嵌套深度）
    pub fn enter(&mut self) { self.depth += 1; }
    pub fn leave(&mut self) { if self.depth > 0 { self.depth -= 1; } }
}

/// 批量决策：对一组调用点统一决策（考虑预算分配）
pub fn batch_decide(heuristic: &mut InlineHeuristic, calls: &[FuncInlineInfo]) -> Vec<InlineDecision> {
    // 先按得分排序，优先内联高价值调用
    let mut indexed: Vec<(usize, u32)> = calls.iter().enumerate()
        .map(|(i, info)| (i, heuristic.score(info))).collect();
    indexed.sort_by_key(|(_, score)| std::cmp::Reverse(*score));

    let mut decisions = vec![InlineDecision::NoInline; calls.len()];
    for (i, _score) in indexed {
        let info = &calls[i];
        let d = heuristic.decide(info);
        heuristic.record(d, info);
        decisions[i] = d;
    }
    decisions
}

#[derive(Debug, Default)]
pub struct InlineStats {
    pub inlined:        usize,
    pub not_inlined:    usize,
    pub maybe:          usize,
    pub total_ir_added: u32,
}
impl InlineStats {
    pub fn format(&self) -> String {
        format!("内联统计: {}已内联 {}未内联 {}边界 新增{}条IR",
            self.inlined, self.not_inlined, self.maybe, self.total_ir_added)
    }
    pub fn inline_rate(&self) -> f64 {
        let total = self.inlined + self.not_inlined + self.maybe;
        if total == 0 { 0.0 } else { self.inlined as f64 / total as f64 * 100.0 }
    }
}
