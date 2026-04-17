#!/bin/bash
# ═══════════════════════════════════════════════════════════════════
# Nova 编译器 一键 Smoke Test
# 包含: 自举固定点验证 + 回归测试套件
# 用法: bash smoke_test.sh
# ═══════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
SEED="$ROOT/种子/rust生成器"
STAGE_DIR="$ROOT/阶段1_自举启动"
KERN="$ROOT/../内核"
TEST_DIR="$KERN/工具/测试框架"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'
PASS=0
FAIL=0

ok()   { echo -e "  ${GREEN}✓${NC} $1"; PASS=$((PASS+1)); }
fail() { echo -e "  ${RED}✗${NC} $1"; FAIL=$((FAIL+1)); }
info() { echo -e "${YELLOW}▶${NC} $1"; }

# ─── 第1步: 自举固定点验证 ───
info "第1步: 自举固定点验证 (cargo run --release)"
cd "$SEED"
OUTPUT=$(cargo run --release 2>&1)

if echo "$OUTPUT" | grep -q "fixed_point: true"; then
    ok "fixed_point: true"
else
    fail "fixed_point 失败"
    echo "$OUTPUT" | tail -10
fi

S2="$STAGE_DIR/阶段2_编译器"
S3="$STAGE_DIR/阶段3_编译器"

if [ -f "$S2" ] && [ -f "$S3" ]; then
    S2_SIZE=$(stat -c%s "$S2")
    S3_SIZE=$(stat -c%s "$S3")
    S2_SHA=$(sha256sum "$S2" | cut -d' ' -f1)
    S3_SHA=$(sha256sum "$S3" | cut -d' ' -f1)

    if [ "$S2_SIZE" = "$S3_SIZE" ]; then
        ok "Stage2/Stage3 大小一致: $S2_SIZE bytes"
    else
        fail "Stage2/Stage3 大小不一致: S2=$S2_SIZE S3=$S3_SIZE"
    fi

    if [ "$S2_SHA" = "$S3_SHA" ]; then
        ok "Stage2/Stage3 SHA256 一致: ${S2_SHA:0:16}..."
    else
        fail "Stage2/Stage3 SHA256 不一致"
    fi
else
    fail "Stage2 或 Stage3 编译器文件不存在"
fi

# 提取模块数和函数数
MOD_COUNT=$(echo "$OUTPUT" | grep -oP 'module_count:\s*\K\d+' || echo "0")
if [ "$MOD_COUNT" -gt 10000 ]; then
    ok "模块函数数: $MOD_COUNT"
else
    fail "模块函数数异常: $MOD_COUNT"
fi

# ─── 第2步: 回归测试 ───
info "第2步: 回归测试套件 (88 用例)"
cd "$TEST_DIR"
TEST_OUTPUT=$("$S2" . 2>&1)

TEST_PASS=$(echo "$TEST_OUTPUT" | grep -oP '通过:\s*\K\d+' | head -1 || echo "0")
TEST_TOTAL=$(echo "$TEST_OUTPUT" | grep -oP '通过:\s*\d+/\K\d+' | head -1 || echo "0")
TEST_FAIL=$((TEST_TOTAL - TEST_PASS))

if [ "$TEST_PASS" -ge 80 ]; then
    ok "回归测试: $TEST_PASS/$TEST_TOTAL 通过"
else
    fail "回归测试: $TEST_PASS/$TEST_TOTAL 通过 (要求≥80)"
fi

if [ "$TEST_FAIL" -le 4 ]; then
    ok "失败数: $TEST_FAIL (≤4, 已知VM缓冲区限制)"
else
    fail "失败数: $TEST_FAIL (>4, 存在新回归)"
    echo "$TEST_OUTPUT" | grep "^\[" | head -10
fi

# ─── 第3步: Stage2 编译器可用性 ───
info "第3步: Stage2 编译器基本可用性"
if [ -x "$S2" ]; then
    ok "Stage2 编译器可执行"
else
    fail "Stage2 编译器不可执行"
fi

S2_FILE_SIZE=$(stat -c%s "$S2" 2>/dev/null || echo 0)
if [ "$S2_FILE_SIZE" -gt 1000000 ]; then
    ok "Stage2 大小合理: $S2_FILE_SIZE bytes"
else
    fail "Stage2 大小异常: $S2_FILE_SIZE bytes"
fi

# ─── 报告 ───
echo ""
echo "════════════════════════════════════════"
TOTAL=$((PASS+FAIL))
if [ "$FAIL" -eq 0 ]; then
    echo -e "${GREEN}  全部通过: $PASS/$TOTAL${NC}"
    echo "════════════════════════════════════════"
    exit 0
else
    echo -e "${RED}  通过: $PASS/$TOTAL  失败: $FAIL${NC}"
    echo "════════════════════════════════════════"
    exit 1
fi
