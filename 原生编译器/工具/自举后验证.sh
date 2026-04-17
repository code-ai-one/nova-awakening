#!/usr/bin/env bash
set -euo pipefail

SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPILER_ROOT="$(cd "$SELF_DIR/.." && pwd)"
WORKSPACE_ROOT="$(cd "$COMPILER_ROOT/.." && pwd)"
BOOT_ROOT="$WORKSPACE_ROOT/分离链式自举"
STAGE_DIR="$BOOT_ROOT/阶段1_自举启动"
STAGE3="$STAGE_DIR/阶段3_编译器"
STAGE2="$STAGE_DIR/阶段2_编译器"
SMOKE_SCRIPT="$BOOT_ROOT/smoke_test.sh"
OUTPUT_DIR="${TMPDIR:-/tmp}/nova_stable_verify"
TARGET="linux"
PROFILE=""
RUN_SELF_TEST=0
REFRESH_BOOTSTRAP=0
VERBOSE=0
COMPILER_BIN=""
declare -a TARGETS=()

die() {
    printf '%s\n' "$*" >&2
    exit 1
}

usage() {
    printf '%s\n' \
        "用法: bash 原生编译器/工具/自举后验证.sh [选项] <模块文件或目录>..." \
        "" \
        "选项:" \
        "  --bootstrap          先运行分离链式自举/smoke_test.sh，刷新稳定编译器" \
        "  --self-test          模块编译验证后，再运行稳定编译器 --self-test" \
        "  --target <name>      编译目标，默认 linux" \
        "  --profile <name>     指定验证时使用的 profile" \
        "  --output-dir <dir>   编译产物输出目录，默认 /tmp/nova_stable_verify" \
        "  --verbose            打印实际执行命令" \
        "  -h, --help           显示帮助"
}

abspath() {
    if command -v realpath >/dev/null 2>&1; then
        realpath "$1"
        return
    fi
    python3 - <<'PY' "$1"
import os
import sys
print(os.path.realpath(sys.argv[1]))
PY
}

pick_compiler() {
    if [[ -x "$STAGE3" ]]; then
        printf '%s\n' "$STAGE3"
        return 0
    fi
    if [[ -x "$STAGE2" ]]; then
        printf '%s\n' "$STAGE2"
        return 0
    fi
    return 1
}

safe_name() {
    printf '%s' "$1" | sed 's/[^[:alnum:]_.-]/_/g'
}

run_cmd() {
    if [[ "$VERBOSE" -eq 1 ]]; then
        printf '[cmd]'
        printf ' %q' "$@"
        printf '\n'
    fi
    "$@"
}

make_temp_project() {
    local profile_value="$1"
    local entry_module_value="${2:-}"
    local project_file
    project_file="$(mktemp "${TMPDIR:-/tmp}/nova_verify_project.XXXXXX.nova")"
    printf '项目名称: 临时验证\n项目类型: compiler_core\n默认Profile: %s\n' "$profile_value" > "$project_file"
    if [[ -n "$entry_module_value" ]]; then
        printf '入口模块: %s\n' "$entry_module_value" >> "$project_file"
    fi
    printf '入口符号: none\n' >> "$project_file"
    printf '%s\n' "$project_file"
}

compile_dir() {
    local input="$1"
    local abs
    local rel
    local out
    local project_file
    abs="$(abspath "$input")"
    [[ -d "$abs" ]] || die "目录不存在: $input"
    [[ "$abs" == "$WORKSPACE_ROOT"* ]] || die "只支持验证工作区内路径: $input"
    rel="$abs"
    if [[ "$abs" == "$WORKSPACE_ROOT/"* ]]; then
        rel="${abs#$WORKSPACE_ROOT/}"
    fi
    mkdir -p "$OUTPUT_DIR"
    out="$OUTPUT_DIR/$(safe_name "$rel")"
    project_file="$(make_temp_project "${PROFILE:-linux-user}")"
    local -a cmd=("$COMPILER_BIN" "$abs" "--compile" "--module-graph" "--project" "$project_file" "--target" "$TARGET" "-o" "$out")
    if [[ -n "$PROFILE" ]]; then
        cmd+=("--profile" "$PROFILE")
    fi
    run_cmd "${cmd[@]}"
    rm -f "$project_file"
    printf '[ok] 目录验证通过: %s -> %s\n' "$input" "$out"
}

compile_file() {
    local input="$1"
    local abs
    local dir
    local base
    local rel
    local out
    local manifest
    local project_file
    local backup=""
    local status=0
    abs="$(abspath "$input")"
    [[ -f "$abs" ]] || die "文件不存在: $input"
    [[ "$abs" == "$WORKSPACE_ROOT"* ]] || die "只支持验证工作区内路径: $input"
    dir="$(dirname "$abs")"
    base="$(basename "$abs")"
    rel="$abs"
    if [[ "$abs" == "$WORKSPACE_ROOT/"* ]]; then
        rel="${abs#$WORKSPACE_ROOT/}"
    fi
    mkdir -p "$OUTPUT_DIR"
    out="$OUTPUT_DIR/$(safe_name "$rel")"
    manifest="$dir/_manifest.txt"
    project_file="$(make_temp_project "${PROFILE:-linux-user}" "$base")"
    if [[ -f "$manifest" ]]; then
        backup="$(mktemp "${TMPDIR:-/tmp}/nova_manifest_backup.XXXXXX")"
        cp "$manifest" "$backup"
    fi
    printf '%s' "$base" > "$manifest"
    local -a cmd=("$COMPILER_BIN" "$abs" "--compile" "--module-graph" "--project" "$project_file" "--target" "$TARGET" "-o" "$out")
    if [[ -n "$PROFILE" ]]; then
        cmd+=("--profile" "$PROFILE")
    fi
    set +e
    run_cmd "${cmd[@]}"
    status=$?
    set -e
    if [[ -n "$backup" ]]; then
        cp "$backup" "$manifest"
        rm -f "$backup"
    else
        rm -f "$manifest"
    fi
    rm -f "$project_file"
    if [[ "$status" -ne 0 ]]; then
        return "$status"
    fi
    printf '[ok] 模块验证通过: %s -> %s\n' "$input" "$out"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --bootstrap)
            REFRESH_BOOTSTRAP=1
            ;;
        --self-test)
            RUN_SELF_TEST=1
            ;;
        --target)
            shift
            [[ $# -gt 0 ]] || die "--target 需要参数"
            TARGET="$1"
            ;;
        --profile)
            shift
            [[ $# -gt 0 ]] || die "--profile 需要参数"
            PROFILE="$1"
            ;;
        --output-dir)
            shift
            [[ $# -gt 0 ]] || die "--output-dir 需要参数"
            OUTPUT_DIR="$1"
            ;;
        --verbose)
            VERBOSE=1
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            TARGETS+=("$1")
            ;;
    esac
    shift
done

if [[ "$REFRESH_BOOTSTRAP" -eq 1 ]]; then
    [[ -f "$SMOKE_SCRIPT" ]] || die "缺少自举验证脚本: $SMOKE_SCRIPT"
    run_cmd bash "$SMOKE_SCRIPT"
fi

COMPILER_BIN="$(pick_compiler || true)"
[[ -n "$COMPILER_BIN" ]] || die "未找到稳定编译器，请先执行: bash 分离链式自举/smoke_test.sh"
printf '稳定编译器: %s\n' "$COMPILER_BIN"

if [[ "${#TARGETS[@]}" -eq 0 && "$RUN_SELF_TEST" -eq 0 ]]; then
    usage
    exit 1
fi

for target_path in "${TARGETS[@]}"; do
    if [[ -d "$target_path" ]]; then
        compile_dir "$target_path"
    else
        compile_file "$target_path"
    fi
done

if [[ "$RUN_SELF_TEST" -eq 1 ]]; then
    (
        cd "$COMPILER_ROOT"
        run_cmd "$COMPILER_BIN" "--self-test"
    )
    printf '[ok] 稳定编译器自检通过\n'
fi

printf '完成\n'
