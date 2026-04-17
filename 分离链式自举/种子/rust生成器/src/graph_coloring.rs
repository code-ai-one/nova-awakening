#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 图着色寄存器分配
/// 对比 register_allocator.rs 的线性扫描，图着色能产生更优解
/// 适用于离线编译（非 JIT），时间成本换代码质量

use std::collections::{HashMap, HashSet, BTreeSet, VecDeque};

/// 干涉图节点（代表一个虚拟寄存器）
#[derive(Debug, Clone)]
pub struct IgNode {
    pub vreg:       u32,
    pub neighbors:  HashSet<u32>,  // 干涉的其他虚拟寄存器
    pub color:      Option<u8>,    // 分配的物理寄存器（None = 未着色/溢出）
    pub spill_cost: f64,           // 溢出代价（越大越不应溢出）
    pub move_related: bool,        // 是否参与 copy 指令（合并候选）
}

impl IgNode {
    pub fn new(vreg: u32) -> Self {
        IgNode { vreg, neighbors: HashSet::new(), color: None, spill_cost: 1.0, move_related: false }
    }
    pub fn degree(&self) -> usize { self.neighbors.len() }
}

/// 干涉图
pub struct InterferenceGraph {
    nodes:       HashMap<u32, IgNode>,
    reg_count:   usize,  // 可用物理寄存器数量（k）
    pub spilled: Vec<u32>,   // 溢出的虚拟寄存器
}

impl InterferenceGraph {
    pub fn new(reg_count: usize) -> Self {
        InterferenceGraph { nodes: HashMap::new(), reg_count, spilled: vec![] }
    }

    /// 添加虚拟寄存器节点
    pub fn add_node(&mut self, vreg: u32) {
        self.nodes.entry(vreg).or_insert_with(|| IgNode::new(vreg));
    }

    /// 添加干涉边（vreg_a 和 vreg_b 同时活跃，不能用同一寄存器）
    pub fn add_interference(&mut self, a: u32, b: u32) {
        if a == b { return; }
        self.add_node(a); self.add_node(b);
        self.nodes.get_mut(&a).unwrap().neighbors.insert(b);
        self.nodes.get_mut(&b).unwrap().neighbors.insert(a);
    }

    /// 设置溢出代价（调用频率越高 = 溢出代价越大）
    pub fn set_spill_cost(&mut self, vreg: u32, cost: f64) {
        if let Some(n) = self.nodes.get_mut(&vreg) { n.spill_cost = cost; }
    }

    /// 简化阶段（Chaitin-Briggs）：找低度节点，压栈
    fn simplify(&self) -> Vec<u32> {
        let k = self.reg_count;
        let mut stack = vec![];
        let mut removed: HashSet<u32> = HashSet::new();
        let mut degrees: HashMap<u32, usize> = self.nodes.iter().map(|(&v, n)| (v, n.degree())).collect();

        let mut changed = true;
        while changed {
            changed = false;
            // 找一个度数 < k 的节点
            let candidate = degrees.iter()
                .filter(|(v, _)| !removed.contains(v))
                .filter(|(_, &d)| d < k)
                .min_by(|(_, &da), (_, &db)| da.cmp(&db))
                .map(|(&v, _)| v);

            if let Some(v) = candidate {
                stack.push(v);
                removed.insert(v);
                // 更新邻居的度数
                if let Some(node) = self.nodes.get(&v) {
                    for &nb in &node.neighbors {
                        if !removed.contains(&nb) {
                            *degrees.get_mut(&nb).unwrap() -= 1;
                        }
                    }
                }
                changed = true;
            }
        }

        // 剩余节点（度数 >= k）：溢出候选，按代价从低到高排序
        let mut spill_candidates: Vec<u32> = degrees.keys()
            .filter(|v| !removed.contains(v))
            .copied().collect();
        spill_candidates.sort_by(|&a, &b| {
            let ca = self.nodes[&a].spill_cost;
            let cb = self.nodes[&b].spill_cost;
            ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
        });
        stack.extend(spill_candidates);
        stack
    }

    /// 着色阶段：从栈中弹出节点，依次着色
    fn color(&mut self, stack: Vec<u32>) {
        for vreg in stack.iter().rev() {
            let used_colors: HashSet<u8> = self.nodes[vreg].neighbors.iter()
                .filter_map(|nb| self.nodes.get(nb).and_then(|n| n.color))
                .collect();
            // 找第一个可用颜色
            let color = (0..self.reg_count as u8).find(|c| !used_colors.contains(c));
            if let Some(c) = color {
                self.nodes.get_mut(vreg).unwrap().color = Some(c);
            } else {
                // 无法着色：溢出
                self.spilled.push(*vreg);
            }
        }
    }

    /// 执行完整的图着色分配
    pub fn allocate(&mut self) -> HashMap<u32, Option<u8>> {
        let stack = self.simplify();
        self.color(stack);
        self.nodes.iter().map(|(&v, n)| (v, n.color)).collect()
    }

    /// 尝试合并 copy 相关节点（George 合并条件）
    pub fn try_coalesce(&mut self, a: u32, b: u32) -> bool {
        // George 条件：a 的每个高度邻居要么也是 b 的邻居，要么度数 < k
        let k = self.reg_count;
        if let (Some(na), Some(nb)) = (self.nodes.get(&a), self.nodes.get(&b)) {
            let can_coalesce = na.neighbors.iter().all(|&nb_v| {
                nb.neighbors.contains(&nb_v) || self.nodes[&nb_v].degree() < k
            });
            if can_coalesce {
                // 合并：把 b 的所有干涉边转移到 a
                let b_neighbors: Vec<u32> = nb.neighbors.clone().into_iter().collect();
                for &bn in &b_neighbors {
                    self.add_interference(a, bn);
                }
                self.nodes.remove(&b);
                return true;
            }
        }
        false
    }

    pub fn stats(&self) -> ColoringStats {
        let total = self.nodes.len() + self.spilled.len();
        let colored = self.nodes.values().filter(|n| n.color.is_some()).count();
        let max_neighbors = self.nodes.values().map(|n| n.neighbors.len()).max().unwrap_or(0);
        ColoringStats { total, colored, spilled: self.spilled.len(), max_degree: max_neighbors }
    }
}

#[derive(Debug)]
pub struct ColoringStats {
    pub total:      usize,
    pub colored:    usize,
    pub spilled:    usize,
    pub max_degree: usize,
}
impl ColoringStats {
    pub fn format(&self) -> String {
        format!("图着色: {}个虚拟寄存器 →{}着色 {}溢出 最大度数{} ({:.0}%效率)",
            self.total, self.colored, self.spilled, self.max_degree,
            if self.total > 0 { self.colored as f64 / self.total as f64 * 100.0 } else { 0.0 })
    }
}

/// 从活跃性分析结果构建干涉图
pub fn build_interference_graph(
    live_sets: &HashMap<u32, Vec<u32>>,  // 程序点 → 活跃变量列表
    reg_count: usize,
) -> InterferenceGraph {
    let mut ig = InterferenceGraph::new(reg_count);
    for (_, live_vars) in live_sets {
        // 同一程序点活跃的所有变量对之间添加干涉边
        for (i, &a) in live_vars.iter().enumerate() {
            ig.add_node(a);
            for &b in live_vars.iter().skip(i + 1) {
                ig.add_interference(a, b);
            }
        }
    }
    ig
}
