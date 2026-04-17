<div align="center">

# Nova 原生中文编译器

**世界首个自举成功的中文原生编程语言编译器**

[![License](https://img.shields.io/badge/许可证-AGPL--3.0%20%2B%20木兰公共版-blue.svg)](NOTICE)
[![Bootstrap](https://img.shields.io/badge/自举-Stage2%3D%3DStage3-brightgreen.svg)](纯血原生自举方案.md)
[![Modules](https://img.shields.io/badge/模块-517-green.svg)](原生编译器/)
[![Language](https://img.shields.io/badge/语言-中文原生-red.svg)]()

</div>

## 🌟 Nova 是什么

Nova 是一个**从零构建**的原生中文编程语言编译器，具备完整的自举能力：

- **中文一等公民**：关键字、标识符、错误消息、注释全部中文
- **字节级自举**：Stage-0(Rust种子) → Stage-1 → Stage-2 == Stage-3（逐字节相同）
- **原生代码生成**：直接输出 x86_64 ELF，无需 LLVM / GCC / 外部运行时
- **16KB 静态产物**：编译出的二进制极小，适合嵌入式和容器冷启动
- **AI 原生**：86/86 AI 算子验证通过，内置浮点/SIMD intrinsics

## 🏗️ 项目结构

```
原生编译器/          ← 权威源码（编译器全链路）
  编译器/词法/       ← 词法分析器
  编译器/语法/       ← 语法分析器
  编译器/语义/       ← 语义分析器
  IR/               ← 中间表示 + 优化器
  平台/代码生成/     ← x86_64 代码生成器
  内存/             ← 内存管理 + GC
分离链式自举/        ← 4阶段自举链
  阶段0_种子编译器/  ← Rust种子引导
  阶段1_自举启动/    ← Stage-1/2/3 产物
纯净版系统/          ← AI原生操作系统
  中枢/             ← 内核
  感知/             ← 驱动层
  外联/             ← 网络栈
  引导/             ← 启动引导
nova标准库/          ← 标准库（模糊逻辑等）
benchmarks/          ← 性能基准测试
```

## ⚡ 性能

| 基准 | Nova/C 比值 | 说明 |
|------|-------------|------|
| fib35 | 3-4x | 递归斐波那契 |
| loop_sum | 2x | 循环求和 |
| collatz | 1.6x | 考拉兹猜想 |
| AI算子 | 86/86 | 全部验证通过 |

> 无寄存器分配、无SIMD代码生成下的性能。R3寄存器分配后预计 Nova/C 1.3-1.5x。

## 🔗 自举链

```
Stage-0 (Rust种子) → Stage-1 → Stage-2 → Stage-3
                                        ↑
                              diff_bytes = 0 (逐字节相同)
```

验证命令：
```bash
cd 分离链式自举/阶段0_种子编译器
python3 自举入口.nova
# 期望: fixed_point=true, diff_bytes=0, convergence_rounds=2
```

## 📜 许可证

**双轨授权策略**：

- **开源版**：AGPL-3.0 + 木兰公共版（MulanPubL-2.0）双许可证
  - 个人学习、研究、教学免费使用
  - 强传染性：修改/分发/网络服务必须开源
- **商业授权**：任何商业使用需书面授权
  - 详见 [NOTICE](NOTICE)

## 🛡️ 安全

- 所有提交 GPG 签名验证
- SSH Ed25519 认证
- noreply 邮箱隐私保护

## 🗺️ 路线图

- [x] 字节级自举固定点
- [x] AI 86/86 算子验证
- [x] 浮点参数类型推断
- [x] SIMD 7 原语 intrinsics
- [ ] 寄存器分配 (R3)
- [ ] SIMD 循环向量化
- [ ] SSA 优化管线
- [ ] GC 垃圾回收
- [ ] 去 Rust 种子（词法/语法/后端 Nova 重写）
- [ ] 纯净版系统闭环

## ☕ 赞助

如果 Nova 对你有帮助，欢迎赞助支持开发：

**加密货币**（冷钱包，安全收款）：

| 链 | 地址 |
|-----|------|
| BTC | `1GBy4btiex2AvYrcXGEtr9gsSWwPZ1z4ho` |
| ETH / USDT (ERC20) / BNB (BEP20) / MATIC | `0x13a0E27B6C0b363144686126Dd9e6d68AA32225A` |
| TRX / USDT (TRC20) | `TVdzMZEgbTLm4hx5dpZ13uhxbVYVMDwsnS` |
| SOL | `EjDz1JYf2a348j2vDsxaE5k417hnGTtyeYZ6Npc9V4qG` |

**Open Collective**: [opencollective.com/nova-awakening](https://opencollective.com/nova-awakening)

## 💬 联系

- GitHub Issues: [提交问题](https://github.com/code-ai-one/nova-awakening/issues)
- 商业授权: Issues 标注 `[商业授权]`
