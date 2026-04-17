#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 寄存器分配器 (Linear Scan Register Allocation)
/// 实现线性扫描算法（比图染色快，适合JIT场景）
/// 支持：x86-64 通用寄存器 / 浮点寄存器 / 溢出管理

use std::collections::{HashMap, BTreeMap};

/// x86-64 通用寄存器集合（不包含 rsp/rbp，这两个有特殊用途）
pub const GPR_CALLER_SAVED: &[u8] = &[
    0,  // rax
    1,  // rcx
    2,  // rdx
    6,  // rsi
    7,  // rdi
    8,  // r8
    9,  // r9
    10, // r10
    11, // r11
];

pub const GPR_CALLEE_SAVED: &[u8] = &[
    3,  // rbx
    12, // r12
    13, // r13
    14, // r14
    15, // r15
];

/// 虚拟寄存器到物理寄存器的映射结果
#[derive(Debug, Clone)]
pub struct Allocation {
    pub vreg:   u32,    // 虚拟寄存器 ID
    pub preg:   Option<u8>,  // 物理寄存器（None = 溢出到栈）
    pub spill_slot: Option<i32>,  // 溢出槽偏移（相对 rbp）
}

impl Allocation {
    pub fn reg(vreg: u32, preg: u8) -> Self { Allocation { vreg, preg: Some(preg), spill_slot: None } }
    pub fn spill(vreg: u32, slot: i32) -> Self { Allocation { vreg, preg: None, spill_slot: Some(slot) } }
    pub fn is_spilled(&self) -> bool { self.preg.is_none() }
}

/// 活跃区间：[start, end)
#[derive(Debug, Clone)]
pub struct LiveInterval {
    pub vreg:  u32,
    pub start: u32,    // 第一次定义的程序点
    pub end:   u32,    // 最后一次使用的程序点
    pub hint:  Option<u8>,  // 建议使用的物理寄存器（来自调用约定）
}

impl LiveInterval {
    pub fn new(vreg: u32, start: u32, end: u32) -> Self {
        LiveInterval { vreg, start, end, hint: None }
    }
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// 线性扫描分配器
pub struct LinearScanAllocator {
    available_regs: Vec<u8>,
    spill_offset:   i32,
    spill_step:     i32,  // 每个溢出槽的大小（字节，通常8）
}

impl LinearScanAllocator {
    pub fn new(regs: &[u8]) -> Self {
        LinearScanAllocator {
            available_regs: regs.to_vec(),
            spill_offset: -8,
            spill_step: 8,
        }
    }

    pub fn with_callee_saved() -> Self {
        let mut regs = GPR_CALLER_SAVED.to_vec();
        regs.extend_from_slice(GPR_CALLEE_SAVED);
        Self::new(&regs)
    }

    /// 执行线性扫描分配
    pub fn allocate(&mut self, intervals: &mut Vec<LiveInterval>) -> Vec<Allocation> {
        // 按开始点排序
        intervals.sort_by_key(|i| i.start);

        let mut active: Vec<usize> = vec![];  // 活跃区间的索引（按结束点排序）
        let mut free_regs: Vec<u8> = self.available_regs.clone();
        let mut allocations: Vec<Allocation> = vec![];

        for (idx, interval) in intervals.iter().enumerate() {
            // 过期旧区间
            let mut new_active = vec![];
            for &ai in &active {
                if intervals[ai].end <= interval.start {
                    // 过期：释放寄存器
                    if let Some(alloc) = allocations.iter().find(|a| a.vreg == intervals[ai].vreg) {
                        if let Some(preg) = alloc.preg {
                            free_regs.push(preg);
                        }
                    }
                } else {
                    new_active.push(ai);
                }
            }
            active = new_active;

            // 分配寄存器
            let alloc = if let Some(hint) = interval.hint {
                // 优先使用建议寄存器
                if let Some(pos) = free_regs.iter().position(|&r| r == hint) {
                    let preg = free_regs.remove(pos);
                    Allocation::reg(interval.vreg, preg)
                } else {
                    self.try_allocate_or_spill(interval, &mut free_regs)
                }
            } else {
                self.try_allocate_or_spill(interval, &mut free_regs)
            };

            allocations.push(alloc);

            // 按结束点插入活跃集合
            let pos = active.iter().position(|&ai| intervals[ai].end >= interval.end).unwrap_or(active.len());
            active.insert(pos, idx);
        }

        allocations
    }

    fn try_allocate_or_spill(&mut self, interval: &LiveInterval, free_regs: &mut Vec<u8>) -> Allocation {
        if !free_regs.is_empty() {
            let preg = free_regs.remove(0);
            Allocation::reg(interval.vreg, preg)
        } else {
            // 溢出到栈
            let slot = self.spill_offset;
            self.spill_offset -= self.spill_step;
            Allocation::spill(interval.vreg, slot)
        }
    }

    /// 需要多少栈空间用于溢出
    pub fn spill_frame_size(&self) -> usize {
        if self.spill_offset < -8 {
            (-8 - self.spill_offset) as usize
        } else { 0 }
    }
}

/// 分配结果摘要
pub fn allocation_summary(allocs: &[Allocation]) -> AllocationStats {
    let total = allocs.len();
    let spilled = allocs.iter().filter(|a| a.is_spilled()).count();
    let reg_dist: HashMap<u8, usize> = allocs.iter()
        .filter_map(|a| a.preg).fold(HashMap::new(), |mut m, r| {
            *m.entry(r).or_default() += 1; m
        });
    AllocationStats { total, spilled, in_reg: total - spilled, reg_dist }
}

#[derive(Debug)]
pub struct AllocationStats {
    pub total:   usize,
    pub in_reg:  usize,
    pub spilled: usize,
    pub reg_dist: HashMap<u8, usize>,  // 每个物理寄存器被使用的次数
}

impl AllocationStats {
    pub fn format(&self) -> String {
        format!("寄存器分配: {}个虚拟寄存器 → {}在寄存器 {}溢出 ({:.0}%溢出率)",
            self.total, self.in_reg, self.spilled,
            if self.total > 0 { self.spilled * 100 / self.total } else { 0 }
        )
    }
    pub fn most_used_reg(&self) -> Option<u8> {
        self.reg_dist.iter().max_by_key(|(_, c)| *c).map(|(&r, _)| r)
    }
}

/// 从 Nova IR 函数中提取活跃区间（简化版：线性扫描顺序）
pub fn extract_intervals_from_function(func_instructions: &[(u32, u32, u32)]) -> Vec<LiveInterval> {
    // func_instructions: (程序点, def虚拟寄存器, use虚拟寄存器)
    let mut intervals: HashMap<u32, LiveInterval> = HashMap::new();
    for &(point, def, uses) in func_instructions {
        if def > 0 {
            intervals.entry(def).or_insert_with(|| LiveInterval::new(def, point, point + 1)).end = point + 1;
        }
        if uses > 0 {
            let e = intervals.entry(uses).or_insert_with(|| LiveInterval::new(uses, 0, 0));
            if e.end < point + 1 { e.end = point + 1; }
        }
    }
    intervals.into_values().collect()
}
