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
# 两个环境变量的作用：
#   GITUI_GALLERY=1     打开组件画廊（主窗口的覆盖层）
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
    nsh run >"$home_dir/run-$name.log" 2>&1 &
  echo $! >"$home_dir/run-$name.pid"
)

# 先等构建完成、应用进程出现，再等界面稳定后截图。
# 固定 sleep 不可靠：冷构建要 60s 以上，热构建只要几秒，截图会落在两种时刻。
for _ in $(seq 1 180); do
  if pgrep -f "nsh-app-nsh-app" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
if ! pgrep -f "nsh-app-nsh-app" >/dev/null 2>&1; then
  echo "应用进程在 180s 内没有出现，看日志：${home_dir#"$root"/}/run-$name.log" >&2
  exit 1
fi
sleep "$settle_seconds"
# 已知限制：这里截的是整屏，而 `screencapture -x` 抓的是当时最前面的窗口。
# 本环境拿不到窗口 id（Apple Events 未授权），也无法把 gitui 提到前台，
# 因此当编辑器/浏览器在前台时，截到的是它们而不是 gitui。
# 产物只能人工确认是 gitui 窗口后才可作为证据。
screencapture -x "$out"
echo "截图：${out#"$root"/}"

if [ -f "$home_dir/run-$name.pid" ]; then
  kill "$(cat "$home_dir/run-$name.pid")" 2>/dev/null || true
  rm -f "$home_dir/run-$name.pid"
fi
echo "已退出应用；日志在 ${home_dir#"$root"/}/run-$name.log"
