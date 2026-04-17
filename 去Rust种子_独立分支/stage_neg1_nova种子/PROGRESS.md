# Nova 种子 mini 编译器 · 进度

## 已完成
- [x] 目录结构建立
- [x] 分析 backend_seed.rs: 1672行 = Lexer(200) + Parser(450) + Codegen(900) + 关键字表(100)
- [x] **`词法.nova` 实现 + 7/7 自测通过** (对齐 backend_seed.rs::Lexer)
  - 支持所有 Token 类型: 关键字/标识符/数字(含0x16进制)/字符串/运算符/分隔符
  - 中文关键字映射 (函数/定义/可变/返回/如果/否则/当/真/假/空 等)
  - UTF-8 多字节标识符
  - BOM 跳过
  - 行注释 `//`
  - 转义字符串 `\n`/`\t`/`\\`
  - 词法正确性: fib35源码 → 31个tokens，与Rust版一致

## 待完成
- [~] **`语法.nova` 骨架扩展** (2026-04-17)
  - ✅ 主框架: parser_初始化/peek/advance/expect
  - ✅ parse_program / parse_stmt 入口
  - ✅ parse_func_def / parse_var_def (含可变) / parse_return
  - ✅ parse_if (含 else / else if 链) / parse_while
  - ✅ parse_expr / parse_term / parse_primary (+-*/)
  - ✅ parse_import / parse_struct_def
  - ✅ continue / break / throw 基础识别
  - [ ] parse_match / parse_try (复杂流程)
  - [ ] 字符串转义 / 列表字面量 / 字典字面量
  - [ ] 逻辑/位运算 & 比较运算 (&& || ! == != < > <= >=)
  - [ ] 索引访问 obj[k] / 成员访问 obj.field
  - [ ] 自测 vs backend_seed.rs Parser AST 对比
- [ ] `后端.nova` 实现 (对齐 Codegen::compile_program, ~900行 → 估计 1000-1400行 Nova)

## ⚠ 2026-04-17 重大认知更新
**纯血原生自举不需要完成此目录!**

详见 `@/home/cch/桌面/新觉醒/纯血原生自举方案.md`

**正确方案**: Reproducible Bootstrap (GHC/SBCL 工业方案)
- 已 checked in `/分离链式自举/永久种子/nova_seed_stage3.bin` (7.5MB)
- 已验证 byte-identical 引导 `bash 从种子引导.sh`
- 零 Rust 依赖

**此目录现为**: 教学级演示 / 替代方案研究, **不再是必要路径**.
继续填充仍有教学价值, 但非战略必需.
- [ ] 字节级对比验证工具: 对同一段Nova源码, Nova-mini vs Rust-backend_seed 产物比较
- [ ] 集成到自举链: 让 Rust 种子调用 Nova-mini (通过执行产物) 替代 backend_seed.rs

## 估期
| 阶段 | 预计工期 |
|------|---------|
| 语法.nova | 1-1.5 天 |
| 后端.nova | 2-2.5 天 |
| 字节级对齐+调试 | 1-2 天 |
| 集成+验证 | 0.5-1 天 |
| **总计** | **4.5-7 天** |

## 风险
1. **字节级对齐**: Rust 版 codegen 有一些微妙实现细节 (寄存器分配策略、立即数编码等)，逐字节对齐需要仔细debug
2. **性能**: Nova 当前 2.23x 慢于 gcc-O2，lexer+parser+codegen 跑在 Nova 上会比 Rust 版慢，但这是可接受的编译期成本
3. **引导循环**: Nova-mini 需要用当前的 Nova 编译器编译，循环依赖已存在并稳定
