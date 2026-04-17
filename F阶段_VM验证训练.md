# Nova 纯净系统·虚拟机验证训练 · 阶段F (30 todo)

**目的**: 在虚拟机环境部署纯净版系统，验证+调试+训练AI

---

## F.1 虚拟机基础（10 todo）

### F.1.1 · VM平台选择
| 选项 | 优缺点 | 推荐度 |
|------|--------|--------|
| QEMU | 开源/跨平台/成熟 | ⭐⭐⭐⭐⭐ |
| KVM | 硬件加速/Linux原生 | ⭐⭐⭐⭐ |
| VirtualBox | 用户友好/图形化 | ⭐⭐⭐ |
| 自研VM | 完全可控/工作量大 | ⭐⭐ |

**选择**: QEMU + KVM 组合 (开发阶段) → 自研VM (最终阶段)

### F.1.2-F.1.10 · VM能力
- 虚拟机镜像构建脚本（Nova纯净版 bootable ISO）
- 启动流程调试（bootloader→kernel→AI）
- 串口调试输出
- 网络配置（virtio-net）
- 磁盘配置（virtio-blk）
- CPU/内存配置（NUMA感知）
- GPU直通（vfio-pci）
- 快照管理（qcow2增量）
- 自动化测试集成

---

## F.2 系统调试（10 todo）

### F.2.1 · 内核panic捕获
```
机制:
  1. 注册panic handler
  2. 捕获寄存器状态+栈回溯
  3. 串口输出
  4. 进入调试shell（若可能）
```

### F.2.2-F.2.10 · 调试能力
- 内存泄漏检测（AddressSanitizer移植）
- 死锁检测（锁依赖图分析）
- 性能profiling（eBPF + 采样）
- AI行为调试（决策可视化）
- 浏览器兼容性测试（网站库）
- 压力测试（Locust风格）
- 稳定性测试（7×24小时连续运行）
- 崩溃恢复（checkpoint/restore）
- 故障注入测试（chaos engineering）

---

## F.3 AI训练流水线（10 todo）

### F.3.1 · 基础模型训练
```
模型: Nova-Brain-Base（DTSN架构）
规模: 
  Base: 1B参数
  Large: 7B参数
  XL: 70B参数
数据: 
  中英文混合（8:2）
  代码+数学+逻辑
```

### F.3.2-F.3.10 · 训练流程
- 系统使用数据收集（用户同意+隐私保护）
- 个性化微调（LoRA/联邦学习）
- 强化学习调优（RLHF + RLAIF）
- 多模态训练（视觉+语言+代码）
- 模型压缩（量化+剪枝+蒸馏）
- 模型蒸馏（teacher→student）
- 部署更新（Blue-green / Canary）
- A/B测试
- 持续训练pipeline（online learning）

---

## 阶段F关键里程碑

### M1 · 首次成功启动 (VM)
- 纯Nova编译的bootloader能加载Nova内核
- 内核能初始化基本设备
- 串口输出 "纯净系统已启动"

### M2 · 第一次浏览器渲染
- 浏览器引擎加载并渲染简单HTML
- 证明 **浏览器外衣** 同构正确

### M3 · AI推理上线
- Nova-Brain模型在VM中成功推理
- 证明 **AI即内核** 可行

### M4 · 第一个AI决策参与调度
- AI模型接管进程调度决策
- 证明 **AI驱动系统** 可行

### M5 · 同构闭环验证
- 五层双向转换都能正确完成
- 证明 **闭环公理** 成立

### M6 · 自举VM
- 在VM内部用Nova编译器重新编译自身
- 证明 **完全自洽** 的系统

---

## 交付物

### 系统镜像
- `nova_pure_base.iso` - 最小可引导ISO
- `nova_pure_dev.iso` - 开发版（含编译器+工具）
- `nova_pure_ai.iso` - AI增强版（含Nova-Brain预训练模型）

### 训练模型
- `nova_brain_base.bin` - 1B参数基础模型
- `nova_brain_large.bin` - 7B参数大模型
- `nova_brain_xl.bin` - 70B参数极大模型

### 文档
- 《Nova纯净版系统设计白皮书》
- 《类人脑AI架构论文》
- 《中文AI基因语言规范》
- 《自举编译器形式化证明》

---

## 30 todo分布
| 子阶段 | Todo |
|--------|------|
| F.1 VM基础 | 10 |
| F.2 系统调试 | 10 |
| F.3 AI训练 | 10 |
| **总计** | **30** |
