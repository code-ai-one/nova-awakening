#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova SSA 形式构建器 (Static Single Assignment)
/// 将 Nova IR 从三地址码转换为 SSA 形式
/// 算法：Cytron et al. 1991（基于支配前沿的 φ 函数插入）

use std::collections::{HashMap, HashSet, VecDeque};

/// SSA 值（版本化的变量引用）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SsaVal {
    pub base_var: u32,   // 原始变量 ID
    pub version:  u32,   // SSA 版本号
}
impl SsaVal {
    pub fn new(var: u32, ver: u32) -> Self { SsaVal { base_var: var, version: ver } }
    pub fn format(&self) -> String { format!("v{}_{}", self.base_var, self.version) }
}

/// φ 函数：在 BB 入口处合并来自不同前驱的值
#[derive(Debug, Clone)]
pub struct PhiNode {
    pub target: SsaVal,              // 新定义的 SSA 值
    pub args:   Vec<(u32, SsaVal)>,  // (前驱块ID, 来自该前驱的 SSA 值)
}

/// SSA 基本块
#[derive(Debug, Clone)]
pub struct SsaBlock {
    pub id:    u32,
    pub phis:  Vec<PhiNode>,
    pub insns: Vec<SsaInsn>,
    pub succs: Vec<u32>,
    pub preds: Vec<u32>,
}

/// SSA 指令（三地址码 SSA 形式）
#[derive(Debug, Clone)]
pub enum SsaInsn {
    Assign(SsaVal, SsaRhs),
    Branch(SsaVal, u32, u32),  // (条件, 真块ID, 假块ID)
    Jump(u32),
    Return(Option<SsaVal>),
    Call(Option<SsaVal>, u32, Vec<SsaVal>),  // (result, func_id, args)
}

#[derive(Debug, Clone)]
pub enum SsaRhs {
    Copy(SsaVal),
    Const(i64),
    BinOp(u8, SsaVal, SsaVal),   // (op, lhs, rhs)
    UnOp(u8, SsaVal),
    Load(SsaVal, i32),             // (base, offset)
    Store(SsaVal, i32, SsaVal),   // (base, offset, value)
}

/// SSA 构建上下文
pub struct SsaBuilder {
    versions: HashMap<u32, u32>,    // 变量 → 当前版本号
    stacks:   HashMap<u32, Vec<u32>>,  // 变量 → 版本号栈（重命名时用）
    pub df:   HashMap<u32, HashSet<u32>>,  // 支配前沿
}

impl SsaBuilder {
    pub fn new() -> Self {
        SsaBuilder { versions: HashMap::new(), stacks: HashMap::new(), df: HashMap::new() }
    }

    /// 获取变量的新版本号
    pub fn new_version(&mut self, var: u32) -> u32 {
        let v = self.versions.entry(var).or_default();
        let ver = *v;
        *v += 1;
        self.stacks.entry(var).or_default().push(ver);
        ver
    }

    /// 查询变量当前版本
    pub fn current_version(&self, var: u32) -> Option<u32> {
        self.stacks.get(&var).and_then(|s| s.last().copied())
    }

    /// 弹出变量版本（退出块时）
    pub fn pop_version(&mut self, var: u32) {
        self.stacks.entry(var).or_default().pop();
    }

    /// 创建 SsaVal（使用当前版本）
    pub fn use_var(&self, var: u32) -> SsaVal {
        let ver = self.current_version(var).unwrap_or(0);
        SsaVal::new(var, ver)
    }

    /// 定义变量（创建新版本）
    pub fn def_var(&mut self, var: u32) -> SsaVal {
        let ver = self.new_version(var);
        SsaVal::new(var, ver)
    }

    /// 计算简单 CFG 的支配前沿（基于即时支配树）
    /// idom: 节点ID → 直接支配节点ID（-1 = 无支配者）
    /// preds: 节点ID → 前驱节点ID列表
    pub fn compute_dominance_frontier(&mut self, idom: &HashMap<u32, i64>, preds: &HashMap<u32, Vec<u32>>) {
        self.df.clear();
        for (&b, pred_list) in preds {
            if pred_list.len() >= 2 {
                for &p in pred_list {
                    let mut runner = p as i64;
                    while runner != *idom.get(&b).unwrap_or(&-1) {
                        self.df.entry(runner as u32).or_default().insert(b);
                        runner = *idom.get(&(runner as u32)).unwrap_or(&-1);
                        if runner < 0 { break; }
                    }
                }
            }
        }
    }

    /// 确定哪些块需要 φ 函数（基于支配前沿）
    /// defs: 变量ID → 定义该变量的块ID集合
    pub fn compute_phi_placement(&self, defs: &HashMap<u32, HashSet<u32>>) -> HashMap<u32, HashSet<u32>> {
        // 返回：块ID → 该块需要插入 φ 函数的变量集合
        let mut phi_blocks: HashMap<u32, HashSet<u32>> = HashMap::new();

        for (&var, def_blocks) in defs {
            let mut worklist: VecDeque<u32> = def_blocks.iter().copied().collect();
            let mut visited: HashSet<u32> = HashSet::new();

            while let Some(block) = worklist.pop_front() {
                if let Some(df_set) = self.df.get(&block) {
                    for &df_block in df_set {
                        phi_blocks.entry(df_block).or_default().insert(var);
                        if !visited.contains(&df_block) {
                            visited.insert(df_block);
                            worklist.push_back(df_block);
                        }
                    }
                }
            }
        }
        phi_blocks
    }

    /// 打印 SSA 统计
    pub fn stats(&self) -> SsaStats {
        let total_versions: usize = self.versions.values().map(|&v| v as usize).sum();
        let max_version = self.versions.values().copied().max().unwrap_or(0);
        SsaStats { variables: self.versions.len(), total_versions, max_version }
    }
}

impl Default for SsaBuilder { fn default() -> Self { Self::new() } }

#[derive(Debug)]
pub struct SsaStats {
    pub variables:      usize,
    pub total_versions: usize,
    pub max_version:    u32,
}

impl SsaStats {
    pub fn format(&self) -> String {
        format!("SSA: {}个变量, {}个版本总计, 最高版本={}", self.variables, self.total_versions, self.max_version)
    }
}

/// 验证 SSA 性质（每个 SsaVal 只被定义一次）
pub fn verify_ssa(blocks: &[SsaBlock]) -> Vec<String> {
    let mut defined: HashSet<SsaVal> = HashSet::new();
    let mut errors = vec![];

    for block in blocks {
        for phi in &block.phis {
            if !defined.insert(phi.target) {
                errors.push(format!("SSA违规: {} 被定义多次（在 φ 函数中）", phi.target.format()));
            }
        }
        for insn in &block.insns {
            if let SsaInsn::Assign(target, _) = insn {
                if !defined.insert(*target) {
                    errors.push(format!("SSA违规: {} 被定义多次", target.format()));
                }
            }
        }
    }
    errors
}

/// 从 SSA 形式收集所有定义（用于活跃性分析）
pub fn collect_all_defs(blocks: &[SsaBlock]) -> HashSet<SsaVal> {
    let mut defs = HashSet::new();
    for block in blocks {
        for phi in &block.phis { defs.insert(phi.target); }
        for insn in &block.insns {
            if let SsaInsn::Assign(target, _) = insn { defs.insert(*target); }
        }
    }
    defs
}
