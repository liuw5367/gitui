#!/usr/bin/env python3
"""从设计稿提取每个画板的几何规格。

用法：
    python3 tools/design/geometry.py --board 04          # 打印某板规格
    python3 tools/design/geometry.py --all               # 全部写入 spec/
    python3 tools/design/geometry.py --board 01 --json   # 只输出 JSON

数据来源是 `designs/graft-git/project/` 下的共享样式表与各 `.dc.html` 的内联
style。同一选择器在样式表中可能声明多次（后出现者覆盖前面的），因此取最后一次
声明；`@media` 块只在窄窗口生效，解析时整块剔除，避免把响应式覆盖当成基准值。

实现层把这些数值固化为 `apps/macos/src/design/metrics.rs` 的常量，并由
`apps/macos/src/design/metrics.rs` 中的测试逐个比对；本脚本负责产出对照表，
方便新增画板或改设计时快速核对。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

DESIGN_ROOT = Path(__file__).resolve().parents[2] / "designs" / "graft-git"
PROJECT = DESIGN_ROOT / "project"
CSS_BLOB = PROJECT / "assets" / "0a859d3b7b7fd8e4958bfb63b7af439a.bin"

# 画板编号 -> `.dc.html` 文件名（编号与导出的 PNG 名称一致，见 canvas.json）。
BOARD_FILES: dict[str, str] = {
    "01": "Welcome",
    "02": "WelcomeEmpty",
    "03": "RepoManager",
    "04": "Workspace",
    "05": "HistoryFilter",
    "06": "DiffFocus",
    "07": "CompareRefs",
    "08": "CommitInfo",
    "09": "RefsManager",
    "10": "Changes",
    "11": "Changelists",
    "12": "Agent",
    "13": "AgentPermissions",
    "14": "MergeConflict",
    "15": "Sync",
    "16": "RepositoryStates",
    "17": "Reflog",
    "18": "CommitFiles",
    "19": "BlameHistory",
    "20": "CommandPalette",
    "21": "ContextMenu",
    "22": "Menus",
    "23": "Settings",
    "24": "SettingsGit",
    "25": "SettingsShortcuts",
    "26": "SettingsAbout",
    "27": "SettingsCredentials",
    "28": "SyncFetch",
    "29": "SyncPull",
    "30": "RefsOther",
}

# 关注的选择器与属性：名称 -> (选择器, 属性)
MEASUREMENTS: dict[str, tuple[str, str]] = {
    "titlebar_h": (".titlebar.with-tabs", "height"),
    "toolbar_h": (".toolbar.v2", "height"),
    "sidebar_w": (".sidebar", "width"),
    "statusbar_h": (".statusbar", "height"),
    "side_row_h": (".side-row", "height"),
    "side_head_h": (".side-head", "height"),
    "hist_bar_h": (".hist-bar", "height"),
    "commit_cols_h": (".commit-cols", "height"),
    "commit_row_h": (".commit", "height"),
    "diffbar_h": (".diffbar", "height"),
    "diff_font": (".diffbody", "font-size"),
    "diff_line_h": (".diffbody", "line-height"),
    "welcome_side_w": (".wside", "width"),
    "welcome_repo_h": (".wrepo", "height"),
    "welcome_group_label_h": (".wglabel", "height"),
    "welcome_appmark": (".appmark", "width"),
    "welcome_title_font": (".wtitle", "font-size"),
    "field_h": (".field", "height"),
    "button_h": (".btn", "height"),
    "button_small_h": (".btn.small", "height"),
    "search_w": (".search", "width"),
    "rtab_h": (".rtab", "height"),
}


def strip_media_queries(css: str) -> str:
    """去掉 `@media` 块（它们只在窄窗口下生效）。"""
    out = []
    rest = css
    while True:
        index = rest.find("@media")
        if index < 0:
            out.append(rest)
            break
        out.append(rest[:index])
        after = rest[index:]
        open_brace = after.find("{")
        if open_brace < 0:
            break
        depth = 0
        end = None
        for offset, character in enumerate(after[open_brace:], start=open_brace):
            if character == "{":
                depth += 1
            elif character == "}":
                depth -= 1
                if depth == 0:
                    end = offset + 1
                    break
        if end is None:
            break
        rest = after[end:]
    return "".join(out)


def last_declaration(css: str, selector: str, prop: str) -> str | None:
    """选择器最后一次声明里某个属性的值。"""
    needle = f"{selector}{{"
    last = None
    position = 0
    while True:
        index = css.find(needle, position)
        if index < 0:
            return last
        start = index + len(needle)
        end = css.find("}", start)
        if end < 0:
            return last
        for declaration in css[start:end].split(";"):
            name, _, value = declaration.partition(":")
            if name.strip() == prop:
                last = value.strip()
        position = end


def inline_dimensions(html: str) -> dict[str, float]:
    """画板内联 style 里出现的像素高度/宽度。"""
    found: dict[str, float] = {}
    for value in re.findall(r"(?:width|height):(\d+(?:\.\d+)?)px", html):
        number = float(value)
        found[f"{number:g}px"] = number
    return found


def board_spec(board: str, css: str) -> dict[str, object]:
    """一个画板的规格：选择器测量值 + 内联尺寸 + 标题。"""
    name = BOARD_FILES.get(board, board)
    matches = sorted(PROJECT.glob(f"{name}*.dc.html"))
    if not matches:
        raise SystemExit(f"找不到画板：{board}（按 {name} 查找）")
    html = matches[0].read_text(encoding="utf-8")
    title = re.search(r"<title>([^<]*)</title>", html)
    measurements: dict[str, object] = {}
    for name, (selector, prop) in MEASUREMENTS.items():
        value = last_declaration(css, selector, prop)
        if value is not None:
            measurements[name] = value
    return {
        "board": matches[0].name,
        "title": title.group(1) if title else "",
        "measurements": measurements,
        "inline": inline_dimensions(html),
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--board", help="画板序号或文件名前缀，例如 01、Welcome")
    parser.add_argument("--all", action="store_true", help="处理全部画板并写入 spec/")
    parser.add_argument("--json", action="store_true", help="只输出 JSON")
    args = parser.parse_args(argv)

    if not CSS_BLOB.exists():
        print(f"找不到共享样式表：{CSS_BLOB}", file=sys.stderr)
        return 1
    css = strip_media_queries(CSS_BLOB.read_text(encoding="utf-8"))

    if args.all:
        targets = sorted(BOARD_FILES)
        output = DESIGN_ROOT / "spec"
        output.mkdir(exist_ok=True)
        for target in targets:
            spec = board_spec(target, css)
            path = output / f"{target}.json"
            path.write_text(json.dumps(spec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
            print(f"写入 {path.relative_to(DESIGN_ROOT.parents[1])}")
        return 0

    if not args.board:
        parser.error("需要 --board 或 --all")
    spec = board_spec(args.board, css)
    if args.json:
        print(json.dumps(spec, ensure_ascii=False, indent=2))
        return 0
    print(f"{spec['board']}  {spec['title']}")
    print("测量值：")
    for name, value in spec["measurements"].items():  # type: ignore[union-attr]
        selector, prop = MEASUREMENTS[name]
        print(f"  {name:22} {value:>8}   {selector} {prop}")
    print("内联尺寸：", ", ".join(sorted(spec["inline"])))  # type: ignore[arg-type]
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
