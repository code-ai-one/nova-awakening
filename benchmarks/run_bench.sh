#!/usr/bin/env bash
# Nova vs C(gcc -O2) vs Rust(-O) Benchmark
# 诚实对比自举后的Nova AOT产物性能

set -e
cd "$(dirname "$0")"

NOVA=/home/cch/桌面/新觉醒/分离链式自举/阶段1_自举启动/阶段3_编译器
OUT=./_out
mkdir -p "$OUT"

bench_one() {
    local name=$1
    echo "=== $name ==="

    # C -O2
    gcc -O2 -o "$OUT/${name}_c" "${name}.c"
    # Rust -O (相当于-O2)
    rustc -O -o "$OUT/${name}_rs" "${name}.rs" 2>/dev/null
    # Nova (当前无-O选项, 默认编译)
    "$NOVA" "${name}.nova" --compile -o "$OUT/${name}_nova" > /dev/null 2>&1
    chmod +x "$OUT/${name}_nova"

    measure() {
        local bin=$1
        local best=99999
        for _ in 1 2 3 4 5; do
            # 用date获取ms精度, Nova程序简单到完成<1ms不适用, 加一轮reps内部循环
            local t0=$(date +%s.%N)
            for _ in 1 2 3; do "$bin" > /dev/null; done
            local t1=$(date +%s.%N)
            local t=$(awk "BEGIN{print ($t1-$t0)/3}")
            if awk "BEGIN{exit !($t < $best)}"; then best=$t; fi
        done
        echo "$best"
    }

    C_T=$(measure "$OUT/${name}_c")
    R_T=$(measure "$OUT/${name}_rs")
    N_T=$(measure "$OUT/${name}_nova")
    printf "  %-10s %s s\n" "C/gcc-O2" "$C_T"
    printf "  %-10s %s s\n" "Rust-O" "$R_T"
    printf "  %-10s %s s\n" "Nova" "$N_T"

    # 计算倍率
    echo "  ─── ratios ───"
    awk "BEGIN{printf \"  Nova/C  = %.2fx\\n\", $N_T/$C_T}"
    awk "BEGIN{printf \"  Nova/Rs = %.2fx\\n\", $N_T/$R_T}"
    echo
}

echo "═══ Nova Benchmark ═══"
echo "Date: $(date)"
echo "Host: $(uname -m) $(uname -s)"
echo "CPU:  $(grep -m1 'model name' /proc/cpuinfo 2>/dev/null | cut -d: -f2 | sed 's/^ *//')"
echo

bench_one fib35
bench_one loop_sum
bench_one collatz

echo "═══ Summary ═══"
echo "产物大小 (bytes):"
for n in fib35 loop_sum collatz; do
    for tag in c rs nova; do
        f="$OUT/${n}_${tag}"
        if [ -f "$f" ]; then
            printf "  %-18s %d\n" "${n}_${tag}" "$(stat -c%s "$f")"
        fi
    done
done
