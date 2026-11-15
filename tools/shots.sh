#!/usr/bin/env bash
# 用重定向的 HOME 启动应用并截图，绕开 ~/Library/Caches/nsh 的写权限限制。
#
# 为什么需要重定向：nsh 的开发会话锁写在 $HOME/Library/Caches/nsh/dev-sessions，
# 沙箱或只读缓存目录下会得到 "Operation not permitted"。把 HOME 指到仓库内的
# .nsh-home 后，锁、增量缓存与应用数据都落在工作区内，可写也可清理。
#
# 用法：
#   tools/shots.sh gallery            # 组件画廊（GITUI_GALLERY=1，并关掉欢迎窗口）
#
# 只截应用窗口：先用 tools/winid.swift 取窗口 id，再 screencapture -l 精确截取。
# 整屏截图会截到最前面的别的程序（曾经截到编辑器并误判「界面空白」）。
#
# 两个环境变量的作用：
#   GITUI_GALLERY=1     打开组件画廊（主窗口的覆盖层）
#   GITUI_SYNC=1        启动即打开同步面板（验收弹层用）
#   GITUI_FORM=1        启动即打开新建分支表单（验收表单弹层用）
#   GITUI_MODE=...      启动时的主视图：history / changes / refs / agent
#   GITUI_NO_WELCOME=1  不打开欢迎窗口——它是独立窗口，默认盖在主窗口上，
#                       会把画廊挡在后面，截出来就是一片白
#   tools/shots.sh welcome            # 欢迎页
#   tools/shots.sh <名字> [秒数]      # 自定义名字与「应用出现后」的额外等待（默认 8）
#
# 产物：docs/verify/native/<名字>.png（整屏截图，含桌面背景；只作为行为证据，
# 不作为像素验收基线）。应用在截图后退出。
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
name="${1:?用法: tools/shots.sh <gallery|welcome|名字> [秒数]}"
settle_seconds="${2:-8}"
home_dir="$root/.nsh-home"
out_dir="$root/docs/verify/native"
out="$out_dir/$name.png"

mkdir -p "$home_dir" "$out_dir"

echo "启动应用（HOME=$home_dir）…"
(
  cd "$root/apps/client"
  HOME="$home_dir" \
  GITUI_NO_WELCOME=1 \
  GITUI_GALLERY="$([ "$name" = "gallery" ] && echo 1 || echo 0)" \
  GITUI_SYNC="${GITUI_SYNC:-0}" \
  GITUI_FORM="${GITUI_FORM:-0}" \
  GITUI_MODE="${GITUI_MODE:-history}" \
    nsh run >"$home_dir/run-$name.log" 2>&1 &
  echo $! >"$home_dir/run-$name.pid"
)

# 取窗口 id：最大的那个窗口就是主窗口。
bin="/tmp/winid"
if [ ! -x "$bin" ]; then
  mkdir -p "$root/.swift-cache"
  CLANG_MODULE_CACHE_PATH="$root/.swift-cache" SWIFT_MODULE_CACHE_PATH="$root/.swift-cache" \
    swiftc -O "$root/tools/winid.swift" -o "$bin" || { echo "winid 编译失败" >&2; exit 1; }
fi
# 等窗口出现的上限要按冷构建算：改动 rust 源码后 nsh 会重新编译，实测 60–100s，
# 早先按热构建的 60s 设上限，改完代码必然超时（曾因此误以为「窗口没出现」）。
for _ in $(seq 1 240); do
  window_id="$("$bin" nsh-app --id 2>/dev/null || true)"
  [ -n "${window_id:-}" ] && break
  sleep 1
done
if [ -z "${window_id:-}" ]; then
  echo "取不到 gitui 窗口 id，截图取消。" >&2
  echo "日志：${home_dir#"$root"/}/run-$name.log" >&2
  exit 1
fi
sleep "$settle_seconds"
screencapture -x -l"$window_id" "$out"
echo "截图：${out#"$root"/}（窗口 id $window_id）"

if [ -f "$home_dir/run-$name.pid" ]; then
  kill "$(cat "$home_dir/run-$name.pid")" 2>/dev/null || true
  rm -f "$home_dir/run-$name.pid"
fi
echo "已退出应用；日志在 ${home_dir#"$root"/}/run-$name.log"
