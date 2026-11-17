#!/usr/bin/env bash
# ui 层机械约束：与 src/ui 的 guard_tests 同源，供本地与阶段门禁调用。
#
# 规则
#   A. src/ui 内不得出现字号字面量与十六进制颜色（字号取刻度常量，颜色取自 design）。
#   B. tools/ui-migrated.txt 列出的页面文件不得直接引用 nsh_base、不得出现裸 on_press、
#      不得写字号字面量、不得自带色板、不得自行指定字体族。
#
# 用法：tools/check-ui.sh [仓库根目录]
set -euo pipefail

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
client="$root/apps/client"
ui_dir="$client/src/ui"
migrated_list="$root/tools/ui-migrated.txt"
base_only_list="$root/tools/ui-base-only.txt"

[ -d "$ui_dir" ] || { echo "找不到 $ui_dir" >&2; exit 2; }

status=0
report() {
  local rule="$1" file="$2" detail="$3"
  echo "  [$rule] ${file#"$root"/}: $detail"
  status=1
}

echo "== A. src/ui 的字号与颜色"

while IFS= read -r file; do
  if grep -nE 'text_size\(px\([0-9]' "$file" >/dev/null 2>&1; then
    report A "$file" "字号字面量：$(grep -nE 'text_size\(px\([0-9]' "$file" | head -1 | cut -d: -f1) 行"
  fi
  if grep -nE '#[0-9A-Fa-f]{3,8}\b' "$file" >/dev/null 2>&1; then
    report A "$file" "十六进制颜色：$(grep -nE '#[0-9A-Fa-f]{3,8}\b' "$file" | head -1 | cut -d: -f1) 行"
  fi
done < <(find "$ui_dir" -name '*.rs' -type f)

echo "== B. 已迁移页面的引用与字面量"

if [ -f "$migrated_list" ]; then
  while IFS= read -r relative; do
    case "$relative" in ''|'#'*) continue ;; esac
    file="$client/src/$relative"
    if [ ! -f "$file" ]; then
      report B "$file" "清单里的文件不存在"
      continue
    fi
    if sed 's|//.*||' "$file" | grep -q 'nsh_base::'; then
      report B "$file" "直接引用或调用 nsh_base，应改用 crate::ui"
    fi
    if grep -q '\.on_press(' "$file"; then
      report B "$file" "裸 on_press，应改用 ui::row 或 ui 按钮"
    fi
    if grep -qE 'text_size\(px\([0-9]' "$file"; then
      report B "$file" "字号字面量"
    fi
    if grep -qE '#[0-9A-Fa-f]{3,8}\b' "$file"; then
      report B "$file" "十六进制颜色"
    fi
    if grep -q 'font_family("' "$file"; then
      report B "$file" "自行指定字体族，应改用 ui::MONO 或默认系统字体"
    fi
  done < "$migrated_list"
else
  echo "  （没有 $migrated_list，跳过）"
fi

echo "== C. 已收口基础组件调用的交互文件"

if [ -f "$base_only_list" ]; then
  while IFS= read -r relative; do
    case "$relative" in ''|'#'*) continue ;; esac
    file="$client/src/$relative"
    if [ ! -f "$file" ]; then
      report C "$file" "清单里的文件不存在"
      continue
    fi
    if sed 's|//.*||' "$file" | grep -q 'nsh_base::'; then
      report C "$file" "直接引用或调用 nsh_base，应改用 crate::ui"
    fi
  done < "$base_only_list"
fi

if [ "$status" -eq 0 ]; then
  echo "check-ui: 通过"
else
  echo "check-ui: 有违反项（见上）" >&2
fi
exit "$status"
