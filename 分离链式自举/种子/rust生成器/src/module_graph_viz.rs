#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 模块依赖图可视化
/// 生成 DOT/Graphviz 格式，可用 dot -Tsvg 渲染为 SVG
/// 应用：架构审查 / 循环依赖检测 / 模块热度标注

use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct ModuleNode {
    pub name:      String,   // 模块相对路径
    pub funcs:     usize,    // 函数数量
    pub lines:     usize,    // 代码行数
    pub layer:     String,   // 所属层（IR/编译器/运行时/标准库等）
    pub is_new:    bool,     // 本轮新增
    pub is_stub:   bool,     // 含大量stub
}

#[derive(Debug, Clone)]
pub struct ModuleEdge {
    pub from: String,
    pub to:   String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Copy)]
pub enum EdgeKind {
    Dependency,    // 正常依赖
    Circular,      // 循环依赖（危险）
    Optional,      // 可选依赖
}

#[derive(Default)]
pub struct ModuleGraph {
    pub nodes: HashMap<String, ModuleNode>,
    pub edges: Vec<ModuleEdge>,
}

impl ModuleGraph {
    pub fn new() -> Self { ModuleGraph::default() }

    pub fn add_node(&mut self, name: impl Into<String>, node: ModuleNode) {
        self.nodes.insert(name.into(), node);
    }

    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>, kind: EdgeKind) {
        self.edges.push(ModuleEdge { from: from.into(), to: to.into(), kind });
    }

    /// 从 manifest 文件加载模块图
    pub fn load_from_manifest(manifest_path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(manifest_path)
            .map_err(|e| format!("读取manifest失败: {}", e))?;
        let mut graph = ModuleGraph::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            let name = line.to_string();
            let layer = Self::detect_layer(&name);
            graph.add_node(name.clone(), ModuleNode {
                name: name.clone(),
                layer,
                ..ModuleNode::default()
            });
        }
        Ok(graph)
    }

    fn detect_layer(name: &str) -> String {
        if name.starts_with("IR/") { return "IR层".into(); }
        if name.starts_with("编译器/分析/") { return "分析层".into(); }
        if name.starts_with("编译器/规划/") { return "代码生成层".into(); }
        if name.starts_with("编译器/语义/") { return "语义层".into(); }
        if name.starts_with("编译器/词法/") || name.starts_with("编译器/语法/") { return "前端层".into(); }
        if name.starts_with("编译器/模块系统/") { return "模块系统".into(); }
        if name.starts_with("运行时/JIT") { return "JIT层".into(); }
        if name.starts_with("运行时/") { return "运行时层".into(); }
        if name.starts_with("内存/") { return "内存层".into(); }
        if name.starts_with("平台/GPU/") { return "GPU层".into(); }
        if name.starts_with("平台/") { return "平台层".into(); }
        if name.starts_with("标准库/数学/") { return "数学库".into(); }
        if name.starts_with("标准库/") { return "标准库".into(); }
        if name.starts_with("链接器/") { return "链接器".into(); }
        if name.starts_with("工具/") { return "工具层".into(); }
        if name.starts_with("AI基因/") { return "AI基因".into(); }
        "其他".into()
    }

    /// 检测循环依赖（简单DFS）
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut rec_stack: HashSet<String> = HashSet::new();
        let mut cycles: Vec<Vec<String>> = vec![];
        let mut path: Vec<String> = vec![];

        // 构建邻接表
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        for edge in &self.edges {
            adj.entry(edge.from.clone()).or_default().push(edge.to.clone());
        }

        for node in self.nodes.keys() {
            if !visited.contains(node) {
                self.dfs_cycle(node, &adj, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }
        cycles
    }

    fn dfs_cycle(
        &self, node: &str,
        adj: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = adj.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.dfs_cycle(neighbor, adj, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(neighbor) {
                    // 找到环：从path中提取环
                    if let Some(start) = path.iter().position(|n| n == neighbor) {
                        let cycle: Vec<String> = path[start..].to_vec();
                        cycles.push(cycle);
                    }
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
    }

    /// 按层分组统计
    pub fn layer_stats(&self) -> HashMap<String, (usize, usize, usize)> {
        // 层名 → (模块数, 总函数数, 总行数)
        let mut stats: HashMap<String, (usize, usize, usize)> = HashMap::new();
        for node in self.nodes.values() {
            let entry = stats.entry(node.layer.clone()).or_default();
            entry.0 += 1;
            entry.1 += node.funcs;
            entry.2 += node.lines;
        }
        stats
    }

    /// 生成 DOT 格式输出
    pub fn to_dot(&self, title: &str) -> String {
        let mut out = format!("digraph \"{}\" {{\n", title);
        out += "  rankdir=LR;\n";
        out += "  node [shape=box, style=filled];\n";
        out += "  edge [fontsize=10];\n\n";

        // 按层分组
        let mut layers: HashMap<String, Vec<&ModuleNode>> = HashMap::new();
        for node in self.nodes.values() {
            layers.entry(node.layer.clone()).or_default().push(node);
        }

        // 输出子图
        let layer_colors: &[(&str, &str)] = &[
            ("IR层", "#ffeecc"), ("分析层", "#ccffcc"), ("代码生成层", "#ccccff"),
            ("语义层", "#ffccff"), ("前端层", "#ffe4cc"), ("运行时层", "#e4ffcc"),
            ("标准库", "#e4e4ff"), ("模块系统", "#fff4cc"), ("内存层", "#ffd4d4"),
        ];

        let mut cluster_id = 0;
        for (layer, nodes) in &layers {
            let color = layer_colors.iter()
                .find(|(l, _)| l == &layer.as_str())
                .map(|(_, c)| *c)
                .unwrap_or("#f8f8f8");

            out += &format!("  subgraph cluster_{} {{\n", cluster_id);
            out += &format!("    label=\"{}\";\n", layer);
            out += &format!("    style=filled;\n    color=\"{}\";\n", color);

            for node in nodes {
                let short = node.name.split('/').last().unwrap_or(&node.name);
                let label = if node.funcs > 0 {
                    format!("{}\\n{}函数", short, node.funcs)
                } else {
                    short.to_string()
                };
                let node_color = if node.is_new { "#90EE90" }
                    else if node.is_stub { "#FFB6C1" }
                    else { "white" };
                out += &format!("    \"{}\" [label=\"{}\", fillcolor=\"{}\"];\n",
                    node.name, label, node_color);
            }
            out += "  }\n\n";
            cluster_id += 1;
        }

        // 输出边
        for edge in &self.edges {
            let style = match edge.kind {
                EdgeKind::Circular => "color=red, style=bold",
                EdgeKind::Optional => "style=dashed",
                EdgeKind::Dependency => "color=gray",
            };
            out += &format!("  \"{}\" -> \"{}\" [{}];\n", edge.from, edge.to, style);
        }

        out += "}\n";
        out
    }

    /// 生成简化的文本格式（不需要Graphviz）
    pub fn to_text_summary(&self) -> String {
        let stats = self.layer_stats();
        let mut out = String::from("=== Nova 内核模块架构概览 ===\n\n");
        let mut layers: Vec<(&String, &(usize, usize, usize))> = stats.iter().collect();
        layers.sort_by_key(|(_, (mods, _, _))| std::cmp::Reverse(*mods));
        for (layer, (mods, funcs, lines)) in &layers {
            out += &format!("[{}] {}个模块  {}函数  {}行\n", layer, mods, funcs, lines);
        }
        out += &format!("\n总计: {}个模块  {}函数  {}行\n",
            self.nodes.len(),
            stats.values().map(|(_, f, _)| f).sum::<usize>(),
            stats.values().map(|(_, _, l)| l).sum::<usize>()
        );
        let cycles = self.detect_cycles();
        if cycles.is_empty() {
            out += "\n✓ 无循环依赖\n";
        } else {
            out += &format!("\n⚠️ 发现 {} 个循环依赖\n", cycles.len());
            for (i, cycle) in cycles.iter().take(5).enumerate() {
                out += &format!("  {}. {}\n", i + 1, cycle.join(" → "));
            }
        }
        out
    }

    /// 输出 DOT 文件到磁盘
    pub fn write_dot(&self, path: &Path, title: &str) -> Result<(), String> {
        let dot = self.to_dot(title);
        std::fs::write(path, dot)
            .map_err(|e| format!("写入DOT文件失败: {} ({})", path.display(), e))
    }
}
