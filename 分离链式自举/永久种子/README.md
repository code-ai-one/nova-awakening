# Nova 永久种子 · 纯血原生自举

**2026-04-17 建立**

## 目的
摆脱 Rust 种子, 实现 **Nova 完全自主引导** (类似 GHC/SBCL 的工业级 reproducible bootstrap).

## 文件
- `nova_seed_stage3.bin` — 永久种子 (7553024 字节)
  - SHA256: `b53cd72b841593d457490c00116d5ce2f84a22fece8a1ee47d39f6e8448d352a`
  - 由 2026-04-17 的 Stage-3 binary 冻结产生
  - 等价于主编译器 (`/原生编译器/` 517 模块) 的机器码形式

- `从种子引导.sh` — 启动脚本
  - 4 步流程: 种子 → Stage-1 → Stage-2 → Stage-3 → 验证 byte-identical

## 使用
```bash
bash /home/cch/桌面/新觉醒/分离链式自举/永久种子/从种子引导.sh
```

## 流程
```
nova_seed_stage3.bin
  ↓ 编译 阶段0_种子编译器/
Stage-1
  ↓ 编译 原生编译器/
Stage-2  ═══ (必须 byte-identical) ═══ Stage-3
```

## 优势 (对比 Rust 种子)
| 项 | Rust 种子 | 永久种子 |
|---|---|---|
| 依赖 | rustc 1.70+ + cargo | 无 (仅 bash + cmp) |
| 体积 | 2.7MB binary + 10000+ 行 Rust 源 | 7.5MB binary (已压缩形式) |
| 可重现 | byte-identical ✓ | byte-identical ✓ |
| 中国自主 | ❌ 依赖外部工具链 | ✅ 完全自主 |

## 更新种子
若主编译器源码有意改动 (不只是重编译), 最新 Stage-3 可能与种子不同.
此时需手动更新:
```bash
cp /tmp/nova_new_stage3 ./nova_seed_stage3.bin
```
**注意**: 只有验证 Stage-2==Stage-3 成功后才能更新种子.

## 工业先例
- GHC: 从上一版 GHC binary bootstrap
- SBCL: checked-in image
- Guix: bootstrap seed 50MB
- Rust: 实际也从 previous rustc bootstrap

Nova 现在加入这个工业级家族.

## 废弃 Rust 种子 (可选)
确认永久种子方案稳定后:
1. 标记 `/种子/rust生成器/` 为 legacy
2. 从 CI 移除 Rust 编译步骤
3. 仅保留作为 fallback
