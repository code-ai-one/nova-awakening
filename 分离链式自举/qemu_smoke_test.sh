#!/bin/bash
# ═══════════════════════════════════════════════════════════════════
# Nova 裸机内核 QEMU Smoke Test
# 生成最小 Multiboot2 ELF → QEMU 启动 → 验证串口输出 "Nova OS"
# ═══════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
KERNEL_ELF="/tmp/nova_bare_kernel.elf"
SERIAL_LOG="/tmp/nova_serial.log"
TIMEOUT=3

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${YELLOW}▶${NC} $1"; }
ok()   { echo -e "  ${GREEN}✓${NC} $1"; }
fail() { echo -e "  ${RED}✗${NC} $1"; }

# ─── 第1步: 生成最小 Multiboot2 裸机 ELF ───
info "第1步: 生成最小 Multiboot2 裸机 ELF"

python3 << 'PYEOF'
import struct, sys

# Multiboot v1 header constants
MB1_MAGIC   = 0x1BADB002
MB1_FLAGS   = 0x00010003  # ALIGN + MEMINFO + AOUT_KLUDGE
MB1_CHECK   = (-(MB1_MAGIC + MB1_FLAGS)) & 0xFFFFFFFF

LOAD_ADDR   = 0x100000

# Build flat 32-bit binary (QEMU loads at LOAD_ADDR in 32-bit protected mode)
code = bytearray()

# Multiboot v1 header with AOUT_KLUDGE address fields
# [magic, flags, checksum, header_addr, load_addr, load_end_addr, bss_end_addr, entry_addr]
HEADER_OFF = 0  # header is at the very start
code += struct.pack('<III', MB1_MAGIC, MB1_FLAGS, MB1_CHECK)
header_addr_off = len(code)
# These 5 fields will be patched at the end once we know the total size
code += struct.pack('<IIIII',
    LOAD_ADDR + HEADER_OFF,  # header_addr
    LOAD_ADDR,               # load_addr
    0,                       # load_end_addr (patch later)
    0,                       # bss_end_addr (same as load_end)
    LOAD_ADDR + 32,          # entry_addr (right after header, patch later)
)
ENTRY_OFF = len(code)  # actual entry point

# ── 32-bit startup ──
# cli
code += b'\xFA'
# mov esp, 0x7C00
code += b'\xBC' + struct.pack('<I', 0x7C00)

# ── Clear page tables 0x70000-0x74000 (use safe area above stack) ──
code += b'\x31\xC0'                        # xor eax, eax
code += b'\xB9' + struct.pack('<I', 4096)  # mov ecx, 4096
code += b'\x57'                            # push edi
code += b'\xBF' + struct.pack('<I', 0x70000)  # mov edi, 0x70000
code += b'\xF3\xAB'                        # rep stosd
code += b'\x5F'                            # pop edi

# ── Setup 4-level page tables ──
# PML4[0] @ 0x70000 = 0x71003 (-> PDPT)
code += b'\xC7\x05'
code += struct.pack('<II', 0x70000, 0x71003)
# PDPT[0] @ 0x71000 = 0x72003 (-> PD)
code += b'\xC7\x05'
code += struct.pack('<II', 0x71000, 0x72003)
# PD[0] @ 0x72000 = 0x83 (2MB huge page, P+W+PS)
code += b'\xC7\x05'
code += struct.pack('<II', 0x72000, 0x000083)
# PD[1] @ 0x72008 = 0x200083 (2MB @ 2MB)
code += b'\xC7\x05'
code += struct.pack('<II', 0x72008, 0x200083)

# ── Load CR3 ──
code += b'\xB8' + struct.pack('<I', 0x70000)  # mov eax, 0x70000
code += b'\x0F\x22\xD8'                       # mov cr3, eax

# ── Enable PAE (CR4.PAE) ──
code += b'\x0F\x20\xE0'  # mov eax, cr4
code += b'\x83\xC8\x20'  # or eax, 0x20
code += b'\x0F\x22\xE0'  # mov cr4, eax

# ── Enable long mode (EFER.LME) ──
code += b'\xB9' + struct.pack('<I', 0xC0000080)
code += b'\x0F\x32'                          # rdmsr
code += b'\x0D' + struct.pack('<I', 0x100)   # or eax, 0x100
code += b'\x0F\x30'                          # wrmsr

# ── Enable paging (CR0.PG) ──
code += b'\x0F\x20\xC0'                          # mov eax, cr0
code += b'\x0D' + struct.pack('<I', 0x80000000)  # or eax, 0x80000000
code += b'\x0F\x22\xC0'                          # mov cr0, eax

# ── Load GDT and far jump to 64-bit ──
code += b'\x0F\x01\x15'  # lgdt [disp32]
gdt_ptr_fixup = len(code)
code += struct.pack('<I', 0)  # placeholder

code += b'\xEA'  # far jmp
long_mode_fixup = len(code)
code += struct.pack('<I', 0)  # placeholder
code += struct.pack('<H', 0x08)

# ── 64-bit entry ──
long_mode_offset = len(code)
code += b'\x66\xB8\x10\x00'  # mov ax, 0x10
code += b'\x8E\xD8\x8E\xC0\x8E\xE0\x8E\xE8\x8E\xD0'  # set ds/es/fs/gs/ss
code += b'\x48\xBC' + struct.pack('<Q', 0x80000)  # mov rsp

# ── Serial port init (0x3F8, 115200) ──
def out_b(port, val):
    return b'\x66\xBA' + struct.pack('<H', port) + bytes([0xB0, val, 0xEE])

code += out_b(0x3F9, 0x00)
code += out_b(0x3FB, 0x80)
code += out_b(0x3F8, 0x01)
code += out_b(0x3F9, 0x00)
code += out_b(0x3FB, 0x03)
code += out_b(0x3FA, 0xC7)
code += out_b(0x3FC, 0x0B)

# ── Output "Nova OS\n" ──
code += b'\x66\xBA' + struct.pack('<H', 0x3F8)
for ch in b'Nova OS\n':
    code += bytes([0xB0, ch, 0xEE])

# ── Exit via isa-debug-exit ──
code += b'\x66\xBA' + struct.pack('<H', 0xF4)
code += b'\xB0\x00\xEE'

# halt
code += b'\xFA\xF4\xEB\xFC'

# ── GDT ──
gdt_offset = len(code)
code += b'\x00' * 8
code += bytes([0xFF,0xFF,0x00,0x00,0x00,0x9A,0xAF,0x00])  # code64
code += bytes([0xFF,0xFF,0x00,0x00,0x00,0x92,0xCF,0x00])  # data
gdt_ptr_offset = len(code)
code += struct.pack('<H', 23)
code += struct.pack('<I', LOAD_ADDR + gdt_offset)

# Fixups
struct.pack_into('<I', code, gdt_ptr_fixup, LOAD_ADDR + gdt_ptr_offset)
struct.pack_into('<I', code, long_mode_fixup, LOAD_ADDR + long_mode_offset)

# Patch Multiboot header: load_end_addr, bss_end_addr, entry_addr
total = len(code)
struct.pack_into('<I', code, header_addr_off + 8,  LOAD_ADDR + total)   # load_end_addr
struct.pack_into('<I', code, header_addr_off + 12, LOAD_ADDR + total)   # bss_end_addr
struct.pack_into('<I', code, header_addr_off + 16, LOAD_ADDR + ENTRY_OFF) # entry_addr

with open("/tmp/nova_bare_kernel.elf", "wb") as f:
    f.write(code)

print(f"Generated flat Multiboot kernel: {total} bytes")
print(f"Entry: 0x{LOAD_ADDR + ENTRY_OFF:X}, load range: 0x{LOAD_ADDR:X}-0x{LOAD_ADDR+total:X}")
PYEOF

echo "==="

# ─── 第2步: 验证内核文件 ───
info "第2步: 验证内核文件"
if [ -f "$KERNEL_ELF" ]; then
    KSIZE=$(stat -c%s "$KERNEL_ELF")
    # Check Multiboot v1 magic at offset 0
    MB_MAGIC=$(xxd -l4 -p "$KERNEL_ELF")
    if [ "$MB_MAGIC" = "02b0ad1b" ]; then
        ok "Multiboot v1 魔数正确 ($KSIZE bytes)"
    else
        fail "Multiboot 魔数不正确: $MB_MAGIC"
    fi
else
    fail "内核文件不存在"
    exit 1
fi

# ─── 第3步: QEMU 启动 ───
info "第3步: QEMU 启动 (${TIMEOUT}s 超时)"
rm -f "$SERIAL_LOG"
touch "$SERIAL_LOG"

timeout "$TIMEOUT" qemu-system-x86_64 \
    -kernel "$KERNEL_ELF" \
    -display none \
    -serial file:"$SERIAL_LOG" \
    -no-reboot \
    -m 128M \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    2>/dev/null || true

# ─── 第4步: 验证输出 ───
info "第4步: 验证串口输出"
CONTENT=$(cat "$SERIAL_LOG" 2>/dev/null || echo "")
if [ -n "$CONTENT" ]; then
    echo "  串口内容: '$CONTENT'"
    if echo "$CONTENT" | grep -q "Nova OS"; then
        ok "检测到 'Nova OS' banner"
    else
        fail "未检测到 'Nova OS'"
        xxd "$SERIAL_LOG" | head -5
    fi
else
    fail "串口日志为空 (内核可能 triple-fault)"
    echo "  提示: 检查内核机器码/页表/GDT"
fi

# ─── 报告 ───
echo ""
echo "════════════════════════════════════════"
echo -e "${GREEN}  QEMU Smoke Test 完成${NC}"
echo "════════════════════════════════════════"
