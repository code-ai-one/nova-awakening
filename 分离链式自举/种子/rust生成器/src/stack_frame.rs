#![allow(dead_code, unused_imports, unused_variables, unused_mut)]
/// Nova 栈帧布局管理
/// 计算函数局部变量的栈空间，生成正确的帧指针偏移
/// 支持：x86-64 / AArch64 / RISC-V 三种调用约定

use std::collections::HashMap;

/// 栈分配策略
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AllocStrategy {
    LargestFirst,  // 大变量优先（减少对齐浪费）
    OrderOfUse,    // 按使用顺序（更好的局部性）
    SpillOnly,     // 只分配溢出变量（寄存器分配后使用）
}

/// 变量的栈槽描述
#[derive(Debug, Clone)]
pub struct StackSlot {
    pub var_id:   u32,
    pub offset:   i32,    // 相对 FP 的字节偏移（负数 = 帧内）
    pub size:     u32,    // 字节大小
    pub align:    u32,    // 对齐要求
    pub is_spill: bool,   // 是否为寄存器溢出槽
}

impl StackSlot {
    pub fn new(var_id: u32, offset: i32, size: u32, align: u32) -> Self {
        StackSlot { var_id, offset, size, align, is_spill: false }
    }
    pub fn spill(var_id: u32, offset: i32) -> Self {
        StackSlot { var_id, offset, size: 8, align: 8, is_spill: true }
    }
}

/// 目标平台约定
#[derive(Debug, Clone, Copy)]
pub enum TargetAbi {
    X86_64SystemV,   // Linux/macOS x86-64
    X86_64Win64,     // Windows x64
    AArch64,         // ARM64 AAPCS
    RiscV64,         // RISC-V psABI
}

impl TargetAbi {
    /// 参数传递寄存器数量
    pub fn param_regs(self) -> usize {
        match self { TargetAbi::X86_64SystemV | TargetAbi::AArch64 | TargetAbi::RiscV64 => 6, TargetAbi::X86_64Win64 => 4 }
    }
    /// 栈帧对齐要求（字节）
    pub fn stack_align(self) -> u32 {
        match self { _ => 16 }  // 所有现代平台都要求16字节对齐
    }
    /// 返回地址保存方式（true=推入栈，false=link register）
    pub fn ra_on_stack(self) -> bool {
        matches!(self, TargetAbi::X86_64SystemV | TargetAbi::X86_64Win64)
    }
    /// 帧指针偏移：局部变量从 [FP - size] 开始
    pub fn fp_offset_base(self) -> i32 {
        match self {
            TargetAbi::X86_64SystemV => 0,    // rbp = 帧顶
            TargetAbi::AArch64 => 0,          // x29 = 帧顶
            TargetAbi::RiscV64 => 0,
            _ => 0,
        }
    }
}

/// 栈帧构建器
pub struct FrameBuilder {
    abi:       TargetAbi,
    slots:     HashMap<u32, StackSlot>,
    next_off:  i32,   // 下一个可用偏移（从 -8 开始向下）
    max_args:  u32,   // 需要传递到栈的最大参数数量
    pub saved_regs: Vec<u8>,  // 需要保存的被调用者保存寄存器
}

impl FrameBuilder {
    pub fn new(abi: TargetAbi) -> Self {
        FrameBuilder {
            abi, slots: HashMap::new(), next_off: -8, max_args: 0, saved_regs: vec![]
        }
    }

    /// 分配一个局部变量的栈槽
    pub fn alloc(&mut self, var_id: u32, size: u32, align: u32) -> &StackSlot {
        // 对齐 next_off
        let align = align as i32;
        let offset = if (-self.next_off) % align == 0 {
            self.next_off
        } else {
            let aligned = ((-self.next_off + align - 1) / align) * align;
            -aligned
        };
        let slot = StackSlot::new(var_id, offset, size, align as u32);
        self.next_off = offset - size as i32;
        self.slots.insert(var_id, slot);
        self.slots.get(&var_id).unwrap()
    }

    /// 分配寄存器溢出槽（8字节对齐）
    pub fn alloc_spill(&mut self, var_id: u32) -> i32 {
        let offset = if self.next_off % 8 == 0 { self.next_off } else { self.next_off - (self.next_off % 8).abs() };
        let mut slot = StackSlot::spill(var_id, offset);
        self.next_off = offset - 8;
        self.slots.insert(var_id, slot);
        offset
    }

    /// 注册需要保存/恢复的被调用者保存寄存器
    pub fn save_reg(&mut self, reg: u8) {
        if !self.saved_regs.contains(&reg) {
            self.saved_regs.push(reg);
            let offset = self.next_off;
            self.next_off -= 8;
            // 记录保存位置（通过特殊 var_id 0xFF00 + reg）
            self.slots.insert(0xFF00 + reg as u32, StackSlot { var_id: 0xFF00 + reg as u32, offset, size: 8, align: 8, is_spill: false });
        }
    }

    /// 设置栈传递参数区域大小
    pub fn set_stack_args(&mut self, count: u32) {
        self.max_args = count;
    }

    /// 获取变量的栈槽
    pub fn get_slot(&self, var_id: u32) -> Option<&StackSlot> {
        self.slots.get(&var_id)
    }

    /// 获取变量的帧偏移（相对 FP）
    pub fn offset_of(&self, var_id: u32) -> Option<i32> {
        self.slots.get(&var_id).map(|s| s.offset)
    }

    /// 计算总帧大小（必须满足对齐要求）
    pub fn frame_size(&self) -> u32 {
        let raw = (-self.next_off) as u32;
        let stack_args_size = self.max_args * 8;
        let total = raw + stack_args_size;
        let align = self.abi.stack_align();
        (total + align - 1) / align * align
    }

    /// 生成帧描述（调试输出）
    pub fn describe(&self) -> String {
        let mut slots: Vec<_> = self.slots.values().collect();
        slots.sort_by_key(|s| s.offset);
        let mut out = format!("栈帧 ({} ABI, {}字节):\n", format!("{:?}", self.abi), self.frame_size());
        for slot in &slots {
            let kind = if slot.is_spill { "spill" } else { "local" };
            out += &format!("  [fp{:+4}] var{} {}字节 ({})\n",
                slot.offset, slot.var_id, slot.size, kind);
        }
        out
    }

    /// 统计
    pub fn stats(&self) -> FrameStats {
        let locals = self.slots.values().filter(|s| !s.is_spill && s.var_id < 0xFF00).count();
        let spills = self.slots.values().filter(|s| s.is_spill).count();
        FrameStats { frame_size: self.frame_size(), local_slots: locals, spill_slots: spills, saved_regs: self.saved_regs.len() }
    }
}

#[derive(Debug)]
pub struct FrameStats {
    pub frame_size:  u32,
    pub local_slots: usize,
    pub spill_slots: usize,
    pub saved_regs:  usize,
}
impl FrameStats {
    pub fn format(&self) -> String {
        format!("栈帧: {}字节 ({}局部 {}溢出 {}保存寄存器)",
            self.frame_size, self.local_slots, self.spill_slots, self.saved_regs)
    }
}

/// 快速分配多个变量（按大到小排序以最小化对齐浪费）
pub fn alloc_all(builder: &mut FrameBuilder, vars: &[(u32, u32, u32)]) {
    // vars: (var_id, size, align)
    let mut sorted = vars.to_vec();
    sorted.sort_by_key(|(_, size, _)| std::cmp::Reverse(*size));
    for (var_id, size, align) in sorted {
        builder.alloc(var_id, size, align);
    }
}
