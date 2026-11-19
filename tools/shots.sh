#!/usr/bin/env bash
# 用重定向的 HOME 启动应用并截图，绕开 ~/Library/Caches/nsh 的写权限限制。
#
# 为什么需要重定向：nsh 的开发会话锁写在 $HOME/Library/Caches/nsh/dev-sessions，
# 沙箱或只读缓存目录下会得到 "Operation not permitted"。把 HOME 指到仓库内的
# .tmp/nsh-home 后，锁、增量缓存与应用数据都落在工作区内，可写也可清理。
#
# 但 CARGO_HOME 不跟着改：它决定依赖源码落在哪个 registry 下，路径不同会让 cargo
# 把整套依赖判为变更，开发者的 `nsh run` 与截图脚本会互相作废对方的编译产物。
# 所以这里把它钉在真实 HOME 的 ~/.cargo 上（可用 SHOTS_REAL_HOME 覆盖）。
#
# 用法：
#   tools/shots.sh gallery            # 独立组件画廊窗口（GITUI_GALLERY=1）
#
# 只截应用窗口：先用 tools/winid.swift 取窗口 id，再 screencapture -l 精确截取。
# 整屏截图会截到最前面的别的程序（曾经截到编辑器并误判「界面空白」）。
#
# 环境变量的作用（每个都可单独设，也可以组合）：
#   GITUI_GALLERY=1     打开独立组件画廊窗口
#   GITUI_SYNC=1        启动即打开同步面板（验收弹层用）
#   GITUI_FORM=1        启动即打开新建分支表单（验收表单弹层用）
#   GITUI_MODE=...      启动时的主视图：history / changes / refs / agent
#   GITUI_PALETTE=1     启动即打开命令面板（⌘K 的等价物）
#   GITUI_SETTINGS=1    启动即打开设置窗口（独立窗口）
#   GITUI_SETTINGS_SECTION=general|shortcuts|agent|git|credentials|about
#                       设置窗口初始分区；截图前需清理残留的 nsh-app 子进程
#   GITUI_RECORD=<动作 id>  启动即进入该动作的快捷键录制态（如 palette），
#                       用于核对录制卡的显示位置
#   GITUI_PENDING_KEY=<组合>  录制态下把该组合当「刚按下」回显在录制卡上（如 alt+shift+k）
#   GITUI_RESTORE_CONFIRM=1   启动即打开设置页的「恢复默认快捷键」确认弹层
#   GITUI_INSPECTOR=1   启动即展开右侧检查器（板 08 / 18）
#   GITUI_CONFLICT=1    指定仓库有进行中的冲突时启动即打开冲突面板（板 14）
#   GITUI_CONFLICT_FILES=1   同时打开冲突文件浮层（Tree/List）
#   GITUI_DIRTY=1       在 GITUI_REPO 里制造一处改动，让 Changes/diff 有内容
#   GITUI_NO_WELCOME=1  不打开欢迎窗口，避免截图时选中比画廊更小的欢迎窗口
#   GITUI_WELCOME_FORM=open|clone|init   欢迎页初始分段
#   GITUI_WELCOME_STATE=empty|recent|clone-error
#   GITUI_WELCOME_RECENT=path|branch:path|branch   recent 状态的种子数据
#   tools/shots.sh welcome            # 欢迎页（清空应用数据，保证从空列表开始）
#   GITUI_WELCOME_FORM=clone tools/shots.sh welcome-clone
#   tools/shots.sh <名字> [秒数]      # 自定义名字与「应用出现后」的额外等待（默认 8）
#
# 产物：.tmp/shots/<名字>.png（应用窗口截图；只作为行为证据，
# 不作为像素验收基线）。.tmp/ 已在 .gitignore 中排除，截图不入库。
# 工具缓存：winid、重定向 HOME 与 Swift 模块缓存都在项目内 .tmp/ 下。
# 应用在截图后退出。
#
# 开关只在 debug 构建里生效（apps/client/src/acceptance.rs）：`nsh run` 是 debug，
# 打包出来的应用不读任何 GITUI_*，所以截图脚本必须配 `nsh run` 使用。
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
name="${1:?用法: tools/shots.sh <gallery|welcome|名字> [秒数]}"
settle_seconds="${2:-8}"
home_dir="$root/.tmp/nsh-home"
app_home="$root/.tmp/app-home"
# 真实 HOME 与 cargo home：HOME 指到 .tmp 只是为了让 nsh 的会话锁
# （$HOME/Library/Caches/nsh/dev-sessions）落在可写的工作区内；但 cargo 的
# CARGO_HOME 必须与开发者本人用的一致——否则依赖源码路径不同，cargo 会把整套
# 依赖判为变更，开发者下次 `nsh run` 就要全量重编一次（实测 1m26s，反之亦然）。
real_home="${SHOTS_REAL_HOME:-$HOME}"
cargo_home="${CARGO_HOME:-$real_home/.cargo}"
swift_cache="$root/.tmp/swift-cache"
bin="$root/.tmp/bin/winid"
out_dir="$root/.tmp/shots"
out="$out_dir/$name.png"

mkdir -p "$home_dir" "$app_home" "$swift_cache" "$(dirname "$bin")" "$out_dir"

welcome_shot=0
case "$name" in
  welcome*) welcome_shot=1 ;;
esac

# 欢迎页要求空会话；清空脚本自己的应用数据目录，不碰仓库其它内容。
if [ "$welcome_shot" = 1 ]; then
  case "$app_home" in
    "$root"/.tmp/app-home) rm -rf -- "$app_home" ;;
    *) echo "拒绝清理非预期目录：$app_home" >&2; exit 1 ;;
  esac
fi

# `GITUI_DIRTY=1` 在 GITUI_REPO 指向的仓库里制造一处未提交改动，
# 让 Changes / diff 视图有内容可截（干净的 fixture 里它们是空的）。
if [ "${GITUI_DIRTY:-0}" = "1" ] && [ -n "${GITUI_REPO:-}" ]; then
  if [ -d "$GITUI_REPO" ]; then
    target=""
    for candidate in src/main.txt src/diff.rs README.md; do
      if [ -f "$GITUI_REPO/$candidate" ]; then target="$GITUI_REPO/$candidate"; break; fi
    done
    if [ -n "$target" ]; then
      printf '\n// screenshot fixture change\n' >>"$target"
      echo "已在 $target 制造一处改动"
    else
      echo "GITUI_DIRTY=1 但 $GITUI_REPO 里找不到可改的文件" >&2
    fi
  fi
fi

echo "启动应用（HOME=${home_dir}）…"
(
  cd "$root/apps/client"
  APP_HOME="$app_home" \
  HOME="$home_dir" \
  CARGO_HOME="$cargo_home" \
  GITUI_NO_WELCOME="$([ "$welcome_shot" = 1 ] && echo 0 || echo 1)" \
  GITUI_GALLERY="$([ "$name" = "gallery" ] && echo 1 || echo 0)" \
  GITUI_SYNC="${GITUI_SYNC:-0}" \
  GITUI_FORM="${GITUI_FORM:-0}" \
  GITUI_MODE="${GITUI_MODE:-history}" \
  GITUI_PALETTE="${GITUI_PALETTE:-0}" \
  GITUI_SETTINGS="${GITUI_SETTINGS:-0}" \
  GITUI_SETTINGS_SECTION="${GITUI_SETTINGS_SECTION:-}" \
  GITUI_INSPECTOR="${GITUI_INSPECTOR:-0}" \
  GITUI_CONFLICT="${GITUI_CONFLICT:-0}" \
  GITUI_CONFLICT_FILES="${GITUI_CONFLICT_FILES:-0}" \
  GITUI_WELCOME_FORM="${GITUI_WELCOME_FORM:-}" \
  GITUI_WELCOME_STATE="${GITUI_WELCOME_STATE:-}" \
  GITUI_WELCOME_RECENT="${GITUI_WELCOME_RECENT:-}" \
    nsh run >"$home_dir/run-$name.log" 2>&1 &
  echo $! >"$home_dir/run-$name.pid"
)

# 取窗口 id：主窗口最大；欢迎页、设置页、组件画廊是较小的独立窗口，用 --smallest。
if [ ! -x "$bin" ] || [ "$root/tools/winid.swift" -nt "$bin" ]; then
  CLANG_MODULE_CACHE_PATH="$swift_cache" SWIFT_MODULE_CACHE_PATH="$swift_cache" \
    swiftc -O "$root/tools/winid.swift" -o "$bin" || { echo "winid 编译失败" >&2; exit 1; }
fi
# 欢迎页、设置页与组件画廊都是独立窗口，比主窗口小：用 --smallest 取目标窗口。
window_flag="--id"
case "$name" in
  welcome*|settings*|gallery*) window_flag="--smallest" ;;
esac
# 等窗口出现的上限要按冷构建算：改动 rust 源码后 nsh 会重新编译，实测 60–100s，
# 早先按热构建的 60s 设上限，改完代码必然超时（曾因此误以为「窗口没出现」）。
for _ in $(seq 1 240); do
  window_id="$("$bin" nsh-app "$window_flag" 2>/dev/null || true)"
  [ -n "${window_id:-}" ] && break
  sleep 1
done
if [ -z "${window_id:-}" ]; then
  echo "取不到 gitui 窗口 id，截图取消。" >&2
  echo "日志：${home_dir#"$root"/}/run-$name.log" >&2
  exit 1
fi
sleep "$settle_seconds"
# 独立窗口（欢迎、设置、组件画廊）可能比主窗口晚出现：等待结束后再按"最小窗口"重取一次 id，
# 否则截到的是先出现的主窗口。
if [ "$window_flag" = "--smallest" ]; then
  latest="$("$bin" nsh-app --smallest 2>/dev/null || true)"
  [ -n "${latest:-}" ] && window_id="$latest"
fi
screencapture -x -l"$window_id" "$out"
echo "截图：${out#"$root"/}（窗口 id ${window_id}）"

if [ -f "$home_dir/run-$name.pid" ]; then
  kill "$(cat "$home_dir/run-$name.pid")" 2>/dev/null || true
  rm -f "$home_dir/run-$name.pid"
fi
echo "已退出应用；日志在 ${home_dir#"$root"/}/run-$name.log"
