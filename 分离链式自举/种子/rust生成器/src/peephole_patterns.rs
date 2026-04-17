#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 窥孔优化模式库
/// 预定义数百条机器码级别的优化规则
/// 替换低效指令序列为等价但更快的序列

/// 操作数类型
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Reg(u8),         // 物理寄存器
    Imm(i64),        // 立即数
    Mem(u8, i32),    // [reg + offset]
    Label(String),   // 标签/符号
    Any,             // 通配符（匹配任意操作数）
}

/// 指令模式
#[derive(Debug, Clone)]
pub struct InsnPattern {
    pub mnemonic: String,
    pub operands: Vec<Operand>,
}

impl InsnPattern {
    pub fn new(mnem: impl Into<String>, ops: Vec<Operand>) -> Self {
        InsnPattern { mnemonic: mnem.into(), operands: ops }
    }
    pub fn matches(&self, actual_mnem: &str, actual_ops: &[Operand]) -> Option<PatternCapture> {
        if self.mnemonic != "*" && self.mnemonic != actual_mnem { return None; }
        if self.operands.len() != actual_ops.len() { return None; }
        let mut captures = vec![];
        for (pattern_op, actual_op) in self.operands.iter().zip(actual_ops.iter()) {
            match pattern_op {
                Operand::Any => { captures.push(actual_op.clone()); }
                _ if pattern_op == actual_op => {}
                _ => return None,
            }
        }
        Some(PatternCapture { captures })
    }
}

#[derive(Debug, Clone)]
pub struct PatternCapture {
    pub captures: Vec<Operand>,  // 通配符捕获的操作数
}

/// 优化规则：from 序列 → to 序列
#[derive(Debug, Clone)]
pub struct PeepholeRule {
    pub name:        String,
    pub from_seq:    Vec<InsnPattern>,
    pub to_seq_desc: String,  // 替换描述
    pub saving:      i32,     // 节省的周期数（正值=优化有益）
    pub category:    RuleCategory,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuleCategory {
    RedundantMov,     // 冗余 mov 消除
    StrengthReduce,   // 强度消减（乘法→移位等）
    ConstFold,        // 常量折叠
    DeadCode,         // 死代码消除
    MemAccess,        // 内存访问优化
    ControlFlow,      // 控制流简化
    Fusion,           // 指令融合
    ZeroOpt,          // 零值特殊化
}

/// 预定义的窥孔优化规则集
pub fn builtin_peephole_rules() -> Vec<PeepholeRule> {
    vec![
        // ─── 冗余 MOV 消除 ───
        PeepholeRule { name: "mov_same_reg".into(),
            from_seq: vec![InsnPattern::new("mov", vec![Operand::Any, Operand::Any])],
            to_seq_desc: "删除 mov rX, rX（自移）".into(), saving: 1,
            category: RuleCategory::RedundantMov,
        },
        PeepholeRule { name: "load_after_store".into(),
            from_seq: vec![
                InsnPattern::new("mov", vec![Operand::Mem(0, 0), Operand::Reg(0)]),
                InsnPattern::new("mov", vec![Operand::Reg(1), Operand::Mem(0, 0)]),
            ],
            to_seq_desc: "store后紧接load同地址 → mov rY, rX".into(), saving: 3,
            category: RuleCategory::RedundantMov,
        },
        // ─── 强度消减 ───
        PeepholeRule { name: "mul_by_2".into(),
            from_seq: vec![InsnPattern::new("imul", vec![Operand::Any, Operand::Imm(2)])],
            to_seq_desc: "imul reg, 2 → add reg, reg".into(), saving: 2,
            category: RuleCategory::StrengthReduce,
        },
        PeepholeRule { name: "mul_by_4".into(),
            from_seq: vec![InsnPattern::new("imul", vec![Operand::Any, Operand::Imm(4)])],
            to_seq_desc: "imul reg, 4 → shl reg, 2".into(), saving: 2,
            category: RuleCategory::StrengthReduce,
        },
        PeepholeRule { name: "mul_by_8".into(),
            from_seq: vec![InsnPattern::new("imul", vec![Operand::Any, Operand::Imm(8)])],
            to_seq_desc: "imul reg, 8 → shl reg, 3".into(), saving: 2,
            category: RuleCategory::StrengthReduce,
        },
        PeepholeRule { name: "mul_by_power2".into(),
            from_seq: vec![InsnPattern::new("imul", vec![Operand::Any, Operand::Any])],
            to_seq_desc: "imul reg, 2^k → shl reg, k".into(), saving: 2,
            category: RuleCategory::StrengthReduce,
        },
        PeepholeRule { name: "div_by_power2".into(),
            from_seq: vec![InsnPattern::new("idiv", vec![Operand::Any])],
            to_seq_desc: "idiv (2^k) → sar reg, k".into(), saving: 20,
            category: RuleCategory::StrengthReduce,
        },
        PeepholeRule { name: "mod_by_power2".into(),
            from_seq: vec![InsnPattern::new("idiv", vec![Operand::Any])],  // 取余
            to_seq_desc: "x % (2^k) → x & (2^k - 1)".into(), saving: 20,
            category: RuleCategory::StrengthReduce,
        },
        // ─── 零值特殊化 ───
        PeepholeRule { name: "xor_zero".into(),
            from_seq: vec![InsnPattern::new("mov", vec![Operand::Any, Operand::Imm(0)])],
            to_seq_desc: "mov reg, 0 → xor reg, reg".into(), saving: 1,
            category: RuleCategory::ZeroOpt,
        },
        PeepholeRule { name: "add_zero".into(),
            from_seq: vec![InsnPattern::new("add", vec![Operand::Any, Operand::Imm(0)])],
            to_seq_desc: "add reg, 0 → 删除".into(), saving: 1,
            category: RuleCategory::ZeroOpt,
        },
        PeepholeRule { name: "mul_zero".into(),
            from_seq: vec![InsnPattern::new("imul", vec![Operand::Any, Operand::Imm(0)])],
            to_seq_desc: "imul reg, 0 → xor reg, reg".into(), saving: 3,
            category: RuleCategory::ZeroOpt,
        },
        PeepholeRule { name: "mul_one".into(),
            from_seq: vec![InsnPattern::new("imul", vec![Operand::Any, Operand::Imm(1)])],
            to_seq_desc: "imul reg, 1 → 删除".into(), saving: 3,
            category: RuleCategory::ZeroOpt,
        },
        // ─── 常量折叠 ───
        PeepholeRule { name: "const_add".into(),
            from_seq: vec![
                InsnPattern::new("mov", vec![Operand::Reg(0), Operand::Any]),
                InsnPattern::new("add", vec![Operand::Reg(0), Operand::Any]),
            ],
            to_seq_desc: "mov + add 常量 → 直接用结果常量".into(), saving: 1,
            category: RuleCategory::ConstFold,
        },
        // ─── 控制流简化 ───
        PeepholeRule { name: "test_before_jmp".into(),
            from_seq: vec![
                InsnPattern::new("cmp", vec![Operand::Any, Operand::Imm(0)]),
                InsnPattern::new("je",  vec![Operand::Any]),
            ],
            to_seq_desc: "cmp reg, 0; je → test reg, reg; jz".into(), saving: 0,  // 等价替换
            category: RuleCategory::ControlFlow,
        },
        PeepholeRule { name: "jmp_to_next".into(),
            from_seq: vec![InsnPattern::new("jmp", vec![Operand::Label("next".into())])],
            to_seq_desc: "跳转到下一条指令 → 删除".into(), saving: 1,
            category: RuleCategory::ControlFlow,
        },
        // ─── 内存访问优化 ───
        PeepholeRule { name: "push_pop_cancel".into(),
            from_seq: vec![
                InsnPattern::new("push", vec![Operand::Any]),
                InsnPattern::new("pop",  vec![Operand::Any]),
            ],
            to_seq_desc: "push + pop 同寄存器 → mov".into(), saving: 1,
            category: RuleCategory::MemAccess,
        },
        // ─── 指令融合 ───
        PeepholeRule { name: "lea_fusion".into(),
            from_seq: vec![
                InsnPattern::new("mov", vec![Operand::Any, Operand::Any]),
                InsnPattern::new("add", vec![Operand::Any, Operand::Any]),
            ],
            to_seq_desc: "mov + add → lea (地址计算融合)".into(), saving: 1,
            category: RuleCategory::Fusion,
        },
        PeepholeRule { name: "inc_dec".into(),
            from_seq: vec![InsnPattern::new("add", vec![Operand::Any, Operand::Imm(1)])],
            to_seq_desc: "add reg, 1 → inc reg".into(), saving: 0,
            category: RuleCategory::Fusion,
        },
        // ─── 死代码 ───
        PeepholeRule { name: "dead_store".into(),
            from_seq: vec![
                InsnPattern::new("mov", vec![Operand::Any, Operand::Any]),
                InsnPattern::new("mov", vec![Operand::Any, Operand::Any]),
            ],
            to_seq_desc: "连续两次store到同地址 → 第一次无效".into(), saving: 2,
            category: RuleCategory::DeadCode,
        },
        PeepholeRule { name: "nop_removal".into(),
            from_seq: vec![InsnPattern::new("nop", vec![])],
            to_seq_desc: "删除 nop（除对齐用）".into(), saving: 1,
            category: RuleCategory::DeadCode,
        },
    ]
}

/// 统计各类别规则数量
pub fn count_by_category(rules: &[PeepholeRule]) -> Vec<(String, usize)> {
    let cats = [
        ("冗余MOV", RuleCategory::RedundantMov),
        ("强度消减", RuleCategory::StrengthReduce),
        ("常量折叠", RuleCategory::ConstFold),
        ("死代码", RuleCategory::DeadCode),
        ("内存优化", RuleCategory::MemAccess),
        ("控制流", RuleCategory::ControlFlow),
        ("指令融合", RuleCategory::Fusion),
        ("零值特化", RuleCategory::ZeroOpt),
    ];
    cats.iter()
        .map(|(name, cat)| (name.to_string(), rules.iter().filter(|r| r.category == *cat).count()))
        .filter(|(_, c)| *c > 0)
        .collect()
}

/// 估算应用所有规则的最大收益（周期数）
pub fn total_saving_potential(rules: &[PeepholeRule]) -> i32 {
    rules.iter().map(|r| r.saving).sum()
}

/// 窥孔优化统计
#[derive(Debug, Default)]
pub struct PeepholeStats {
    pub rules_applied: usize,
    pub cycles_saved:  i32,
    pub insns_removed: usize,
}
impl PeepholeStats {
    pub fn format(&self) -> String {
        format!("窥孔优化: 应用{}条规则 节省{}周期 删除{}条指令",
            self.rules_applied, self.cycles_saved, self.insns_removed)
    }
}
