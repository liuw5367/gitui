# domain

领域类型与纯解析。`crates/git` 与 `apps/client` 共用的最小集合：这里不运行命令、不读写存储、不依赖 UI，也没有第三方依赖。

## 模块

| 文件 | 内容 |
|---|---|
| `src/lib.rs` | `GitError`、`NetworkFault`、`Commit`、`StatusEntry`、`DiffStat`、`RepoSummary`；状态分类 |
| `src/diff.rs` | 解析 unified diff：`Patch::parse`、行分类与行号、文件统计、原始文本 `Patch::raw` 与可选全文上下文 `Patch::full_context`、并排配对 `split_rows` |
| `src/stage.rs` | 把补丁切成文件与 hunk：`Sections::parse`、`hunk_patch`、`line_patch` |
| `src/conflict.rs` | Git index 三侧内容模型、冲突标记解析、单侧合并、冲突计数 |

## 主要类型

- `GitError`：七类失败，分别是找不到 git、不是仓库、路径不可读、命令失败、进程启动失败、钩子拒绝与输出解析失败。界面按类别选择呈现方式，不解析错误文案。
- `NetworkFault`：网络失败的三种可恢复形态。归类函数 `classify_network_error` 已搬到 `git`（它匹配的是 Git 自己的输出措辞）。
- `Commit`、`StatusEntry`、`DiffStat`、`RepoSummary`：历史、状态、统计与仓库摘要的共用结构。`StatusEntry::is_staged` 与 `is_unstaged` 按 index/worktree 字符判断。
- `Patch`、`DiffLine`、`DiffFile`、`SplitRow`：渲染 diff 需要的行、文件区间与并排行。`Patch::raw` 是默认上下文原文，复制与暂存使用它；`Patch::full_context` 是同一差异的可选全文上下文显示版本。
- `Sections`、`FileSection`、`Hunk`：生成单 hunk 或单行补丁所需的原始片段。
- `conflict::ConflictSides`：Git index stage 1/2/3 的原始字节内容；每一侧均可缺失，用于表达 add/add、delete/modify 等冲突里的文件创建或删除。

## 约定

- 解析器只处理字符串与数据结构，不判断业务语义；diff 解析把无法识别的行归入 `DiffLineKind::Meta`，stage 解析保留 hunk 里的原始行，未识别的前置行并入文件头。
- 生成 hunk 补丁与单行补丁时必须带完整文件头，hunk 头按实际行数重算；单行补丁的旧侧起始行落在目标行上。
- 冲突解析保留 base 段，`merge` 可以取 ours、theirs 或两侧拼接。stage 数据用原始字节，不在领域层假定文本编码。
- 网络失败分类是启发式判断（在 `crates/git` 的 `error` 模块）；没有命中特征串时保持未知，不给出猜测结论。

## 使用方

- [crates/git](../git/README.md)：把 Git 输出解析成这些类型，并在暂存 hunk 或单行时调用 `Sections`。
- [apps/client](../../apps/client/README.md)：直接消费这些类型渲染界面。

## 测试

```sh
cd crates/domain
cargo test
```

当前 17 项通过，覆盖 diff 行分类与行号、hunk 头、并排配对、hunk/单行补丁、冲突解析与单侧合并、状态分类与错误详情的构造（网络失败归类的 3 项测试随函数搬到 `crates/git`）。

## 相关

- [项目 README](../../README.md)
