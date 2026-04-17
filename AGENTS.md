# Nova 原生中文语言编译器

## Repository expectations
- This is a native Chinese-language compiler project. All keywords, identifiers, error messages, and comments must remain in Chinese. Never replace Chinese with English.
- The compiler uses a 4-stage bootstrap chain (Stage-0 → Stage-1 → Stage-2 → Stage-3). Stage-2 and Stage-3 binaries must be byte-identical. Any non-determinism is a bug that must be fixed at the source level — never patch binaries or lower verification standards.
- Before modifying any module, analyze cross-module dependencies: 词法→语法→语义→IR→优化→代码生成→链接. A change in one module may break downstream modules.
- After every code change, run the bootstrap verification to confirm no regression. Record results in MEMORY.md.
- All algorithms must be original and self-evolving. Never copy existing implementations.
- Never delete modules or simplify implementations. Only upgrade/replace with equal or superior alternatives already in place.
- Never skip stages. The three phases must be completed in strict order: (1) compiler completion → (2) pure system compilation → (3) VM runtime verification.

## Commands
- Bootstrap verification: `cd /home/cch/桌面/新觉醒/分离链式自举/阶段0_种子编译器 && python3 自举入口.nova`
- Compiler test: `cd /home/cch/桌面/新觉醒/原生编译器 && python3 Nova.nova --test`
- Zero warnings required: treat any compiler warning as a bug that must be fixed

## Project structure
- `原生编译器/` — Nova compiler pipeline: 词法分析器→语法分析器→语义分析器→IR生成器→优化器→代码生成器→链接器→模块系统
- `分离链式自举/` — 4-stage bootstrap: 阶段0_种子编译器 → 阶段1 → 阶段2 → 阶段3
- `纯净版系统/` — AI-native OS: 内核, 浏览器引擎, AI运行时, 系统服务, 虚拟机接口

## Current status
- Bootstrap version: 4.1.0
- Bootstrap chain: Stage-0(Rust) → Stage-1 → Stage-2 → Stage-3 → verify stage2==stage3
- Known issues: bootstrap verification not yet passing; Stage-2 post-patch is forbidden (must fix real code generation root cause)
