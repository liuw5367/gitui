#!/usr/bin/env python3
"""生成并校验 `docs/design/TOKENS.md` 里的 token 表。

用法：
    python3 tools/design/tokens.py            # 校验表是否与 theme.json 一致
    python3 tools/design/tokens.py --write    # 重写文档里的表
    python3 tools/design/tokens.py --print    # 只打印生成结果

取值只来自 `apps/client/theme.json`（`cssVars.light` / `cssVars.dark`），代码入口
从 `apps/client/src/design/mod.rs` 的 `return token("…")` 反查，两边都不在本脚本里
另抄一份。分组与顺序写在本文件的 `GROUPS`：theme.json 新增键而没在这里分组时，
校验会失败——这是刻意的，新 token 必须顺便回答「它属于哪一层」。

`theme.json` 的颜色与设计稿共享样式表是否同值，由 `design/mod.rs` 的 `theme_tests`
在 `cargo test` 里逐个比对；本脚本只保证文档与 theme.json 不脱节。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
THEME = ROOT / "apps" / "client" / "theme.json"
DESIGN_MOD = ROOT / "apps" / "client" / "src" / "design" / "mod.rs"
DOC = ROOT / "docs" / "design" / "TOKENS.md"

COLORS_BEGIN = "<!-- tokens:colors:begin（由 tools/design/tokens.py 生成，勿手改） -->"
COLORS_END = "<!-- tokens:colors:end -->"
FONTS_BEGIN = "<!-- tokens:fonts:begin（由 tools/design/tokens.py 生成，勿手改） -->"
FONTS_END = "<!-- tokens:fonts:end -->"

# 分组即语义层级：同一组的键在界面上回答同一个问题。顺序就是文档顺序。
GROUPS: list[tuple[str, list[str]]] = [
    (
        "NSH / shadcn 基座",
        [
            "background",
            "foreground",
            "card",
            "card-foreground",
            "popover",
            "popover-foreground",
            "primary",
            "primary-foreground",
            "secondary",
            "secondary-foreground",
            "muted",
            "muted-foreground",
            "accent",
            "accent-foreground",
            "destructive",
            "border",
            "input",
            "ring",
            "sidebar",
            "sidebar-foreground",
            "sidebar-border",
        ],
    ),
    (
        "表面（窗口骨架）",
        ["content", "panel", "panel2", "bar", "titlebar", "tabbar", "status"],
    ),
    ("文本", ["text", "text2", "text3"]),
    ("线条与描边", ["line", "line-soft", "line-strong", "win-border", "overlay"]),
    (
        "交互反馈",
        ["hover", "press", "sel", "sel-strong", "focus", "shadow-menu"],
    ),
    ("强调色", ["blue", "accent-2", "accent-soft", "primary-bg"]),
    ("表单与按钮", ["field", "field-border", "btn", "btn-border"]),
    ("语义状态", ["warn", "danger", "ok", "purple"]),
    (
        "引用、标签与头像",
        [
            "branch-bg",
            "branch-fg",
            "tag-bg",
            "tag-fg",
            "tag-border",
            "remote-fg",
            "avatar-bg",
            "avatar-fg",
        ],
    ),
    (
        "diff 与折叠",
        [
            "add-bg",
            "add-fg",
            "add-word",
            "add-ln",
            "del-bg",
            "del-fg",
            "del-word",
            "del-ln",
            "hunk-bg",
            "hunk-fg",
            "fold-bg",
        ],
    ),
    (
        "代码高亮",
        [
            "code-keyword",
            "code-string",
            "code-comment",
            "code-number",
            "code-type",
            "code-fn",
        ],
    ),
    ("提交图线", ["g1", "g2", "g3", "g4"]),
    ("菜单", ["menu-bg", "menu-line"]),
]

FONT_GROUP = ("字体族", ["mono", "sans"])

ACCESSOR_RE = re.compile(
    r'fn\s+([a-z0-9_]+)\(\)\s*->\s*AdaptiveColor\s*\{\s*return\s+token\("([^"]+)"\);',
    re.S,
)


def read_theme() -> dict[str, dict[str, str]]:
    data = json.loads(THEME.read_text(encoding="utf-8"))
    css = data.get("cssVars")
    if not isinstance(css, dict):
        raise SystemExit("theme.json 缺少 cssVars")
    light = css.get("light")
    dark = css.get("dark")
    if not isinstance(light, dict) or not isinstance(dark, dict):
        raise SystemExit("theme.json 缺少 cssVars.light / cssVars.dark")
    return {"light": light, "dark": dark}


def read_accessors() -> dict[str, str]:
    """token 名 -> `design` 里的函数名。"""
    source = DESIGN_MOD.read_text(encoding="utf-8")
    out: dict[str, str] = {}
    for name, key in ACCESSOR_RE.findall(source):
        out[key] = name
    return out


def check_groups(keys: list[str]) -> None:
    grouped: list[str] = []
    for _, group_keys in GROUPS + [FONT_GROUP]:
        grouped.extend(group_keys)
    duplicates = {key for key in grouped if grouped.count(key) > 1}
    if duplicates:
        raise SystemExit(f"分组里有重复的键：{', '.join(sorted(duplicates))}")
    missing = [key for key in keys if key not in grouped]
    unknown = [key for key in grouped if key not in keys]
    problems = []
    if missing:
        problems.append(
            "theme.json 里这些键没有分组，先在 GROUPS 决定它属于哪一层：\n  "
            + "\n  ".join(missing)
        )
    if unknown:
        problems.append(
            "GROUPS 里这些键已不在 theme.json，删掉分组里的条目：\n  "
            + "\n  ".join(unknown)
        )
    if problems:
        raise SystemExit("\n\n".join(problems))


def cell(value: object) -> str:
    if value is None:
        return "—"
    return f"`{value}`"


def table(group_keys: list[str], theme: dict[str, dict[str, str]], accessors: dict[str, str]) -> str:
    lines = ["| Token | 代码入口 | Light | Dark |", "|---|---|---|---|"]
    for key in group_keys:
        accessor = f"`design::{accessors[key]}()`" if key in accessors else "—"
        light = theme["light"].get(key)
        dark = theme["dark"].get(key)
        lines.append(f"| `{key}` | {accessor} | {cell(light)} | {cell(dark)} |")
    return "\n".join(lines)


def render(theme: dict[str, dict[str, str]], accessors: dict[str, str]) -> tuple[str, str]:
    colors: list[str] = []
    index = 0
    for title, group_keys in GROUPS:
        index += 1
        colors.append(f"#### {index}. {title}\n")
        colors.append(table(group_keys, theme, accessors) + "\n")
    font_title, font_keys = FONT_GROUP
    fonts = [f"| Token | 代码入口 | 值 |", "|---|---|---|"]
    for key in font_keys:
        accessor = f"`design::{accessors[key]}()`" if key in accessors else "—"
        fonts.append(f"| `{key}` | {accessor} | {cell(theme['light'].get(key))} |")
    return "\n".join(colors).rstrip() + "\n", "\n".join(fonts).rstrip() + f"\n\n`{font_title}` 只有一组值，不随明暗切换。\n"


def region(text: str, begin: str, end: str) -> tuple[int, int]:
    start = text.find(begin)
    stop = text.find(end)
    if start < 0 or stop < 0 or stop < start:
        raise SystemExit(f"docs/design/TOKENS.md 缺少标记：{begin} / {end}")
    return start + len(begin), stop


def splice(text: str, begin: str, end: str, body: str) -> str:
    start, stop = region(text, begin, end)
    return text[:start] + "\n\n" + body.rstrip() + "\n\n" + text[stop:]


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="生成并校验 docs/design/TOKENS.md 的 token 表")
    parser.add_argument("--write", action="store_true", help="把生成的表写回 docs/design/TOKENS.md")
    parser.add_argument("--print", dest="echo", action="store_true", help="只打印生成结果")
    args = parser.parse_args(argv)

    theme = read_theme()
    accessors = read_accessors()
    keys = list(theme["light"].keys())
    check_groups(keys)

    orphan = [key for key in theme["dark"] if key not in theme["light"]]
    if orphan:
        raise SystemExit(f"dark 有 light 没有的键：{', '.join(orphan)}")

    colors, fonts = render(theme, accessors)

    if args.echo:
        print(colors)
        print(fonts)
        return 0

    if not DOC.exists():
        raise SystemExit(f"找不到 {DOC}（先建文档，表会填进标记之间）")
    current = DOC.read_text(encoding="utf-8")
    updated = splice(splice(current, COLORS_BEGIN, COLORS_END, colors), FONTS_BEGIN, FONTS_END, fonts)

    if args.write:
        if updated != current:
            DOC.write_text(updated, encoding="utf-8")
            print(f"已更新 {DOC.relative_to(ROOT)}")
        else:
            print("表已是最新")
        return 0

    if updated != current:
        print(
            "docs/design/TOKENS.md 的 token 表与 theme.json 不一致；运行 "
            "`python3 tools/design/tokens.py --write` 重新生成。",
            file=sys.stderr,
        )
        return 1
    print(f"token 表一致（{len(keys)} 个键，{len(accessors)} 个代码入口）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
