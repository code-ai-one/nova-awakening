# 去 Rust 种子 独立分支工作

## 目标
用 Nova 源码替换 `分离链式自举/种子/rust生成器/src/backend_seed.rs`，让 Nova 真正脱离 Rust 独立自举。

## 总体策略
三步走，每步都要保持主链 `fixed_point: true, diff_bytes=0`：

### 第一步：Nova 版 mini 编译器 (当前)
在 `stage_neg1_nova种子/` 实现 Nova 源码版本的词法 + 语法 + codegen：
- `词法.nova` — 对齐 backend_seed.rs 的 Lexer (Token 类型 + tokenize)
- `语法.nova` — 对齐 Parser (parse_program → Vec<Stmt>)
- `后端.nova` — 对齐 Codegen (compile_program → Vec<u8>)

**验证标准**：对同一段 Nova 源码，Nova-mini 和 Rust-backend_seed 产出 **字节完全相同** 的机器码。

### 第二步：字节码搬运退化 Rust 种子
Rust 种子保留的唯一职责：
1. 读取 Nova-mini 预编译的字节码文件
2. 写入 ELF
3. 驱动 Stage-1 → Stage-2 → Stage-3 自举链

### 第三步：完全去 Rust
Nova 源码直接编译成 Stage-0 可执行字节码，Rust 种子被删除。

## 当前状态
- [x] 目录结构建立
- [x] 分析 backend_seed.rs 结构 (1672行 = Lexer 200 + Parser 450 + Codegen 900 + 关键字表 100)
- [ ] 实现 Nova 版 词法.nova
- [ ] 实现 Nova 版 语法.nova
- [ ] 实现 Nova 版 后端.nova
- [ ] 字节级对比验证 (fib35 / loop_sum / collatz / Nova自编译入口)

## 工作纪律
- 保持主链 `分离链式自举/` 不动
- 本目录任何实验都不影响自举
- 字节级对比作为每步验收标准
- 不推测 Rust 行为，必要时用 `rustc -Z unpretty` / objdump 验证
