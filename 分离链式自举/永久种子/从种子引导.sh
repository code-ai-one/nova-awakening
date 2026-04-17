#!/bin/bash
# Nova 纯血原生自举 · 从 checked-in Stage-3 binary 完成 byte-identical 引导
# 不再需要 Rust 工具链
# 
# 前置: /分离链式自举/永久种子/nova_seed_stage3.bin 存在
# 产出: /tmp/nova_new_stage2, /tmp/nova_new_stage3 (应 byte-identical)

set -e

SEED="/home/cch/桌面/新觉醒/分离链式自举/永久种子/nova_seed_stage3.bin"
STAGE0_SRC="/home/cch/桌面/新觉醒/分离链式自举/阶段0_种子编译器"
MAIN_SRC="/home/cch/桌面/新觉醒/原生编译器"

echo "═══ Nova 纯血原生自举 ═══"
echo "永久种子: $SEED"
echo "阶段 0 源: $STAGE0_SRC"
echo "主编译器源: $MAIN_SRC"
echo ""

# Step 1: 用永久种子编译阶段0_种子编译器, 产出新 Stage-1
echo "[Step 1] 种子 → Stage-1 (编译 阶段0_种子编译器)..."
cd "$STAGE0_SRC"
"$SEED" Nova.nova --compile --module-graph -o /tmp/nova_new_stage1 2>&1 | tail -3
chmod +x /tmp/nova_new_stage1

# Step 2: Stage-1 编译主编译器, 产出 Stage-2
echo ""
echo "[Step 2] Stage-1 → Stage-2 (编译 原生编译器)..."
cd "$MAIN_SRC"
/tmp/nova_new_stage1 Nova.nova --compile --module-graph -o /tmp/nova_new_stage2 2>&1 | tail -3
chmod +x /tmp/nova_new_stage2

# Step 3: Stage-2 再编译主编译器, 产出 Stage-3
echo ""
echo "[Step 3] Stage-2 → Stage-3 (再编译 原生编译器)..."
/tmp/nova_new_stage2 Nova.nova --compile --module-graph -o /tmp/nova_new_stage3 2>&1 | tail -3
chmod +x /tmp/nova_new_stage3

# Step 4: 验证 Stage-2 == Stage-3 byte-identical
echo ""
echo "[Step 4] 验证 byte-identical..."
if cmp /tmp/nova_new_stage2 /tmp/nova_new_stage3; then
    echo "✅ Stage-2 == Stage-3 byte-identical"
    echo ""
    echo "大小: $(stat -c%s /tmp/nova_new_stage2) 字节"
    echo "SHA256:"
    sha256sum /tmp/nova_new_stage2 /tmp/nova_new_stage3
else
    echo "❌ Stage-2 != Stage-3, 自举失败"
    exit 1
fi

# Step 5: 可选更新永久种子
echo ""
echo "═══ 自举完成 ═══"
echo "当前永久种子: $SEED ($(stat -c%s $SEED) 字节)"
echo "新 Stage-3:   /tmp/nova_new_stage3 ($(stat -c%s /tmp/nova_new_stage3) 字节)"
if cmp -s "$SEED" /tmp/nova_new_stage3; then
    echo "✓ 新 Stage-3 与永久种子相同, 无需更新"
else
    echo ""
    echo "⚠ 新 Stage-3 与永久种子不同 (改动了编译器源码)"
    echo "  若改动有意, 更新种子: cp /tmp/nova_new_stage3 $SEED"
    echo "  若改动无意, 回滚源码后重试"
fi
