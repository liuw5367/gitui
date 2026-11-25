//! diff 视图与它的显示开关：共享工具栏与 NSH DiffView 正文。
//!
//! 正文自 P2 起交给 `ui::diff_surface`（包装 `nsh_base::DiffView`），等宽 12 / 行高
//! 18、行号与正负行配色都由它负责。本文件只剩工具栏与开关。
//!
//! 补丁在 `git` 边界就解析成 `domain::diff::Patch`，这里只负责展示与开关。
//! 选项与开关动作放在 `shared`：History、Changes、Stash、Blame 和 Reflog 共用同一份显示偏好。

use crate::design::{self, metrics};
use crate::features::SessionHandle;
use crate::i18n;
use domain::diff::Patch;
use nsh_base::DiffHunk;
use nsh_core::prelude::*;

/// diff 的显示开关：布局、上下文折叠/展开、换行与空白处理。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DiffOptions {
    /// 仅显示变更片段；关闭时显示完整文件。
    pub fold: bool,
    /// 长行换行而不是横向截断。
    pub wrap: bool,
    /// 把行尾空格与制表符显示成可见符号。
    pub whitespace: bool,
}

/// diff 的显示状态：并排/合并、上下文、换行与空白选项。
#[derive(Clone, Copy)]
pub(crate) struct DiffState {
    /// 并排（split）还是合并（unified）视图。
    pub(crate) split: Signal<bool>,
    pub(crate) options: Signal<DiffOptions>,
    /// 让 Git 忽略空格与制表符差异；单独持有以免其它显示开关触发 Git 查询。
    pub(crate) ignore_whitespace: Signal<bool>,
}

/// diff 的用例：切换视图与显示开关、文件树的两项开关。
#[derive(Clone)]
pub(crate) struct DiffCommands {
    pub(crate) state: DiffState,
    /// 文件树两项开关写进设置，因此需要设置与它所在目录。
    pub(crate) settings: Signal<store::Settings>,
    pub(crate) store_dir: std::path::PathBuf,
    /// 剪贴板提示等会话级状态。
    pub(crate) session: SessionHandle,
}

impl DiffCommands {
    /// 切换并排/合并视图（原 `set_diff_split`）。
    pub(crate) fn set_diff_split(&self, split: bool) {
        self.state.split.set(split);
    }

    /// 切换 diff 显示开关（`fold` / `wrap` / `whitespace`，原 `toggle_diff_option`）。
    pub(crate) fn toggle_diff_option(&self, which: &'static str) {
        let mut next = self.state.options.get_untracked();
        match which {
            "fold" => next.fold = !next.fold,
            "wrap" => next.wrap = !next.wrap,
            "whitespace" => next.whitespace = !next.whitespace,
            "ignore_whitespace" => {
                self.state
                    .ignore_whitespace
                    .set(!self.state.ignore_whitespace.get_untracked());
                return;
            }
            _ => return,
        }
        self.state.options.set(next);
    }

    /// 设置文件树的整体开关（原 `set_file_tree`）。
    pub(crate) fn set_file_tree(&self, enabled: bool) {
        let mut next = self.settings.get_untracked();
        next.file_tree = enabled;
        self.settings.set(next.clone());
        store::save_settings(&self.store_dir, &next);
    }

    /// 设置文件树的两项开关（原 `set_tree_flag`）。
    pub(crate) fn set_tree_flag(&self, which: &'static str, value: bool) {
        let mut next = self.settings.get_untracked();
        match which {
            "merge_empty_dirs" => next.merge_empty_dirs = value,
            "compact_single_child" => next.compact_single_child = value,
            _ => return,
        }
        self.settings.set(next.clone());
        store::save_settings(&self.store_dir, &next);
    }
}

#[derive(Clone)]
pub(crate) struct FileNavigation {
    pub(crate) current: usize,
    pub(crate) total: usize,
    pub(crate) previous: std::sync::Arc<dyn Fn()>,
    pub(crate) next: std::sync::Arc<dyn Fn()>,
}

pub(crate) fn diff_title_bar(
    language: Signal<SharedString>,
    back_key: &'static str,
    back: std::sync::Arc<dyn Fn()>,
    title: String,
    file_name: Option<String>,
    file_count: usize,
    file_navigation: Option<FileNavigation>,
) -> AnyElement {
    let title_bar = view()
        .w_full()
        .h(px(metrics::DIFF_RETURN_H))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(11.0))
        .border_b_1()
        .border_color(move || design::line())
        .bg(move || design::panel())
        .child(crate::ui::button::button_icon(
            move || i18n::t(language.get().as_str(), back_key),
            "chev-left",
            crate::ui::button::ButtonVariant::Secondary,
            crate::ui::button::ButtonSize::Small,
            false,
            move || back(),
        ))
        .child(
            text(title)
                .flex_1()
                .min_w_0()
                .flex_shrink(1.0)
                .number_of_lines(1)
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || design::text3())
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        );
    let title_bar = if let Some(file_name) = file_name {
        title_bar.child(
            text(file_name)
                .max_w(px(320.0))
                .min_w_0()
                .flex_shrink(1.0)
                .number_of_lines(1)
                .text_size(px(crate::ui::CODE))
                .text_color(move || design::text())
                .text_overflow(nsh_core::EllipsizeMode::Head),
        )
    } else {
        title_bar
    };
    let title_bar = if let Some(navigation) = file_navigation {
        let current = navigation.current + 1;
        let previous = navigation.previous;
        let next = navigation.next;
        title_bar
            .child(crate::ui::button::icon_button_tip(
                "chev-left",
                {
                    let language = language;
                    move || i18n::t(language.get().as_str(), "diff.previousFile")
                },
                crate::ui::button::ButtonSize::Small,
                navigation.current == 0,
                {
                    let language = language;
                    move || i18n::t(language.get().as_str(), "diff.previousFile")
                },
                move || previous(),
            ))
            .child(crate::ui::button::icon_button_tip(
                "chev-right",
                {
                    let language = language;
                    move || i18n::t(language.get().as_str(), "diff.nextFile")
                },
                crate::ui::button::ButtonSize::Small,
                navigation.current + 1 >= navigation.total,
                {
                    let language = language;
                    move || i18n::t(language.get().as_str(), "diff.nextFile")
                },
                move || next(),
            ))
            .child(
                text(format!("{current}/{}", navigation.total))
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
            )
    } else {
        title_bar
    };
    return title_bar
        .child(
            text(move || {
                i18n::with_arguments(
                    language.get().as_str(),
                    "files.changedCount",
                    &[("count", file_count.to_string())],
                )
            })
            .text_size(px(crate::ui::SECONDARY))
            .text_color(move || design::text3()),
        )
        .into_element();
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn diff(
    state: &DiffState,
    cmd: &DiffCommands,
    language: Signal<SharedString>,
    patch: Patch,
    raw_patch: Patch,
    empty_feedback: Option<&'static str>,
    return_title: String,
    file_count: usize,
    return_to_history: std::sync::Arc<dyn Fn()>,
    open_blame: Option<std::sync::Arc<dyn Fn()>>,
    file_name: Option<String>,
    file_navigation: Option<FileNavigation>,
) -> impl IntoElement {
    let title_bar = diff_title_bar(
        language,
        "history.backToList",
        return_to_history,
        return_title,
        file_name,
        file_count,
        file_navigation,
    );

    return view()
        .w_full()
        .flex_1()
        .min_h_0()
        .flex_col()
        .bg(move || design::content())
        .child(title_bar)
        .child(diff_panel(
            state,
            cmd,
            language,
            patch,
            raw_patch,
            empty_feedback,
            open_blame,
            None,
            None,
        ));
}

pub(crate) fn diff_panel(
    state: &DiffState,
    cmd: &DiffCommands,
    language: Signal<SharedString>,
    patch: Patch,
    raw_patch: Patch,
    empty_feedback: Option<&'static str>,
    open_blame: Option<std::sync::Arc<dyn Fn()>>,
    on_ignore_whitespace: Option<std::sync::Arc<dyn Fn(bool)>>,
    staging: Option<crate::ui::diff_surface::StagingActions>,
) -> AnyElement {
    let options = state.options.get();
    let ignore_whitespace = state.ignore_whitespace.get();
    let patch_for_view = if options.fold {
        patch.raw.clone()
    } else {
        patch
            .full_context
            .as_deref()
            .filter(|text| !text.trim().is_empty())
            .unwrap_or(patch.raw.as_str())
            .to_string()
    };
    let copy_hunk = copy_hunk_handler(&patch_for_view, cmd.session.notice, language);
    let display_patch = if options.whitespace {
        show_whitespace(&patch_for_view)
    } else {
        patch_for_view.clone()
    };
    let additions: u32 = raw_patch.files.iter().map(|file| file.additions).sum();
    let deletions: u32 = raw_patch.files.iter().map(|file| file.deletions).sum();
    let change_count = change_group_count(&patch);
    let hunk_position = signal(if change_count > 0 { Some(1_u32) } else { None });
    let handle = crate::ui::diff_surface::DiffHandle::new();
    let split = state.split.get();
    let toggle: std::sync::Arc<dyn Fn(&'static str)> = {
        let cmd = cmd.clone();
        let ignore_whitespace = state.ignore_whitespace;
        std::sync::Arc::new(move |which| {
            cmd.toggle_diff_option(which);
            if which == "ignore_whitespace"
                && let Some(refresh) = on_ignore_whitespace.as_ref()
            {
                refresh(ignore_whitespace.get_untracked());
            }
        })
    };
    let set_split: std::sync::Arc<dyn Fn(bool)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |value| cmd.set_diff_split(value))
    };
    let previous = {
        let handle = handle.clone();
        let position = hunk_position;
        move || {
            if let Some(index) = handle.previous_hunk() {
                position.set(Some(index + 1));
            }
        }
    };
    let next = {
        let handle = handle.clone();
        let position = hunk_position;
        move || {
            if let Some(index) = handle.next_hunk() {
                position.set(Some(index + 1));
            }
        }
    };
    let copy_patch_text = raw_patch.raw.clone();
    let copy_patch_disabled = copy_patch_text.trim().is_empty();
    let copy_patch = {
        let notice = cmd.session.notice;
        let text = copy_patch_text.clone();
        move || crate::shared::os::copy_text(notice, language, text.clone())
    };
    let mut toolbar = view()
        .w_full()
        .h(px(metrics::DIFFBAR_H))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(11.0))
        .border_b_1()
        .border_color(move || design::line())
        .child(crate::ui::icon::icon_view(
            "diff",
            crate::ui::icon::IconSize::Button,
            design::text3,
        ))
        .child(
            text(format!("+{additions}"))
                .font_family(crate::ui::MONO)
                .text_size(px(crate::ui::CODE))
                .text_color(move || design::add_fg()),
        )
        .child(
            text(format!("−{deletions}"))
                .font_family(crate::ui::MONO)
                .text_size(px(crate::ui::CODE))
                .text_color(move || design::del_fg()),
        )
        .child(view().flex_1().min_w_0())
        .child(mode_toggle(language, split, set_split))
        .child(toolbar_separator())
        .child(fold_toggle(language, options, toggle.clone()))
        .child(wrap_toggle(language, options, toggle.clone()))
        .child(space_toggle(language, options, toggle.clone()))
        .child(ignore_whitespace_toggle(
            language,
            ignore_whitespace,
            toggle,
        ))
        .child(toolbar_separator())
        .child(
            text(move || {
                i18n::with_arguments(
                    language.get().as_str(),
                    "diff.changePosition",
                    &[
                        ("current", hunk_position.get().unwrap_or(0).to_string()),
                        ("total", change_count.to_string()),
                    ],
                )
            })
            .text_size(px(crate::ui::CODE))
            .text_color(move || design::text3()),
        )
        .child(crate::ui::button::icon_button_tip(
            "chev-left",
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "diff.previousChange")
            },
            crate::ui::button::ButtonSize::Small,
            change_count == 0,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "diff.previousChange")
            },
            move || previous(),
        ))
        .child(crate::ui::button::icon_button_tip(
            "chev-right",
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "diff.nextChange")
            },
            crate::ui::button::ButtonSize::Small,
            change_count == 0,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "diff.nextChange")
            },
            move || next(),
        ));
    if let Some(open_blame) = open_blame {
        toolbar = toolbar.child(crate::ui::button::button(
            move || i18n::t(language.get().as_str(), "files.blame"),
            crate::ui::button::ButtonVariant::Ghost,
            crate::ui::button::ButtonSize::Small,
            false,
            move || open_blame(),
        ));
    }
    toolbar = toolbar.child(crate::ui::button::icon_button_tip(
        "copy",
        {
            let language = language;
            move || i18n::t(language.get().as_str(), "diff.copy")
        },
        crate::ui::button::ButtonSize::Small,
        copy_patch_disabled,
        {
            let language = language;
            move || i18n::t(language.get().as_str(), "diff.copy")
        },
        move || copy_patch(),
    ));
    let raw_empty = raw_patch.raw.trim().is_empty();
    let view_empty = patch.raw.trim().is_empty();
    return view()
        .w_full()
        .flex_1()
        .min_h_0()
        .flex_col()
        .child(toolbar)
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(dynamic(move || -> AnyElement {
                    if raw_empty {
                        if let Some(key) = empty_feedback {
                            return feedback_state(language, key).into_element();
                        }
                        return empty_state(language).into_element();
                    }
                    if view_empty || change_count == 0 {
                        return no_text_changes(language).into_element();
                    }
                    let layout = if split {
                        crate::ui::diff_surface::Layout::Split
                    } else {
                        crate::ui::diff_surface::Layout::Unified
                    };
                    crate::ui::diff_surface::configured_surface(
                        SharedString::from(display_patch.clone()),
                        layout,
                        options.wrap,
                        !options.fold,
                        &handle,
                        language,
                        copy_hunk.clone(),
                        staging.clone(),
                    )
                    .into_element()
                })),
        )
        .into_element();
}

/// 与 DiffView 的变更导航一致，统计连续的新增／删除行段。
///
/// 全文 patch 只有一个 Git hunk，但可能包含多处被上下文行隔开的实际改动；
/// `\ No newline at end of file` 属于当前改动段，不能把它拆成两个位置。
fn change_group_count(patch: &Patch) -> u32 {
    let mut count = 0_u32;
    let mut in_change = false;
    for line in &patch.lines {
        match line.kind {
            domain::diff::DiffLineKind::Add | domain::diff::DiffLineKind::Remove => {
                if !in_change {
                    count += 1;
                    in_change = true;
                }
            }
            domain::diff::DiffLineKind::Meta if line.text == "\\ No newline at end of file" => {}
            domain::diff::DiffLineKind::Context
            | domain::diff::DiffLineKind::Hunk
            | domain::diff::DiffLineKind::Meta => in_change = false,
        }
    }
    count
}

fn toolbar_separator() -> impl IntoElement {
    return view()
        .w(px(1.0))
        .h(px(18.0))
        .bg(move || design::line_soft());
}

/// 一个图标开关（fold / wrap / whitespace），开启时用强调色。
fn diff_toggle(
    language: Signal<SharedString>,
    name: &'static str,
    hint_key: &'static str,
    active: bool,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    let label_key = match hint_key {
        "fold" if active => "diff.unfold",
        "fold" => "diff.fold",
        "wrap" => "diff.wrap",
        "whitespace" => "diff.showWhitespace",
        _ => "diff.ignoreWhitespace",
    };
    let foreground = std::rc::Rc::new(move || {
        if active {
            design::accent()
        } else {
            design::text3()
        }
    });
    let background = std::rc::Rc::new(move || {
        if active {
            design::accent_soft()
        } else {
            AdaptiveColor::TRANSPARENT
        }
    });
    let icon = if hint_key == "fold" {
        if active { "expand-all" } else { "collapse-all" }
    } else {
        name
    };
    let label = move || i18n::t(language.get().as_str(), label_key);
    return crate::ui::button::icon_button_tuned_tip(
        icon,
        label,
        crate::ui::button::ButtonSize::Small,
        Some(crate::ui::icon::IconSize::Compact),
        Some(foreground),
        Some(background),
        false,
        move || i18n::t(language.get().as_str(), label_key),
        move || toggle(hint_key),
    );
}

fn fold_toggle(
    language: Signal<SharedString>,
    options: DiffOptions,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    return diff_toggle(language, "fold", "fold", options.fold, toggle);
}

fn wrap_toggle(
    language: Signal<SharedString>,
    options: DiffOptions,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    return diff_toggle(language, "wrap", "wrap", options.wrap, toggle);
}

fn space_toggle(
    language: Signal<SharedString>,
    options: DiffOptions,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    return diff_toggle(language, "space", "whitespace", options.whitespace, toggle);
}

fn ignore_whitespace_toggle(
    language: Signal<SharedString>,
    active: bool,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    return diff_toggle(language, "filter", "ignore_whitespace", active, toggle);
}

/// 在 diff 的展示副本中标记空格和 tab，不修改 `Patch::raw`。
fn show_whitespace(patch: &str) -> String {
    let mut shown = String::with_capacity(patch.len());
    for line in patch.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let code_line = (content.starts_with('+') && !content.starts_with("+++"))
            || (content.starts_with('-') && !content.starts_with("---"))
            || content.starts_with(' ');
        if code_line {
            let (prefix, body) = content.split_at(1);
            shown.push_str(prefix);
            for character in body.chars() {
                match character {
                    ' ' => shown.push('·'),
                    '\t' => shown.push('⇥'),
                    _ => shown.push(character),
                }
            }
        } else {
            shown.push_str(content);
        }
        if line.ends_with('\n') {
            shown.push('\n');
        }
    }
    return shown;
}

/// 按原始 patch 的顺序提取 hunk 文本，供 DiffView 的复制动作使用。
fn raw_hunks(patch: &str) -> Vec<String> {
    let mut hunks = Vec::new();
    let mut current: Option<String> = None;
    for line in patch.split_inclusive('\n') {
        if line.starts_with("diff --git ") || line.starts_with("@@ ") {
            if let Some(hunk) = current.take() {
                hunks.push(hunk);
            }
            if line.starts_with("@@ ") {
                current = Some(line.to_string());
            }
        } else if let Some(hunk) = current.as_mut() {
            hunk.push_str(line);
        }
    }
    if let Some(hunk) = current {
        hunks.push(hunk);
    }
    return hunks;
}

/// Build the standard clipboard action used by read-only `DiffView` instances.
pub(crate) fn copy_hunk_handler(
    patch: &str,
    notice: Signal<Option<String>>,
    language: Signal<SharedString>,
) -> std::sync::Arc<dyn Fn(&DiffHunk)> {
    let hunk_patches = raw_hunks(patch);
    return std::sync::Arc::new(move |hunk: &DiffHunk| {
        if let Some(raw) = hunk_patches.get(hunk.index as usize) {
            crate::shared::os::copy_text(notice, language, raw.clone());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{raw_hunks, show_whitespace};

    #[test]
    fn whitespace_display_keeps_patch_headers_and_marks_code_lines() {
        let patch = "diff --git a/src.rs b/src.rs\n@@ -1 +1 @@ fn main()\n-    let old = \"a b\";\n+    let new = \"a\tb\";\n";

        assert_eq!(
            show_whitespace(patch),
            "diff --git a/src.rs b/src.rs\n@@ -1 +1 @@ fn main()\n-····let·old·=·\"a·b\";\n+····let·new·=·\"a⇥b\";\n"
        );
    }

    #[test]
    fn copied_hunks_keep_their_file_header_and_do_not_include_other_files() {
        let patch = "diff --git a/a.rs b/a.rs\nindex 1..2\n@@ -1 +1 @@\n-old\n+new\ndiff --git a/b.ts b/b.ts\nindex 3..4\n@@ -2 +2 @@\n-before\n+after\n";

        assert_eq!(
            raw_hunks(patch),
            vec![
                "@@ -1 +1 @@\n-old\n+new\n",
                "@@ -2 +2 @@\n-before\n+after\n"
            ]
        );
    }
}

/// 空补丁提示（无改动或读取失败）。
fn empty_state(language: Signal<SharedString>) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(60.0))
        .items_center()
        .justify_center()
        .child(
            text(move || i18n::t(language.get().as_str(), "history.diffEmpty"))
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || design::text3()),
        );
}

fn feedback_state(language: Signal<SharedString>, key: &'static str) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(60.0))
        .items_center()
        .justify_center()
        .child(
            text(move || i18n::t(language.get().as_str(), key))
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || design::text3()),
        );
}

/// 有变更元信息但没有可显示的文本 hunk（例如二进制文件或纯重命名）。
fn no_text_changes(language: Signal<SharedString>) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(60.0))
        .items_center()
        .justify_center()
        .child(
            text(move || i18n::t(language.get().as_str(), "diff.noTextChanges"))
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || design::text3()),
        );
}

/// 合并 / 并排切换（设计板 04 的 Split / Unified）。
///
/// 设计稿没有并排 diff 的排版规格，这里按通用做法实现左右两栏、按行对齐，
/// 属于推断而非像素还原。
fn mode_toggle(
    language: Signal<SharedString>,
    split: bool,
    set_split: std::sync::Arc<dyn Fn(bool)>,
) -> impl IntoElement {
    let target_key = if split {
        "diff.switchToUnified"
    } else {
        "diff.switchToSplit"
    };
    let icon = if split { "split" } else { "unified" };
    return crate::ui::button::icon_button_tip(
        icon,
        move || i18n::t(language.get().as_str(), target_key),
        crate::ui::button::ButtonSize::Small,
        false,
        move || i18n::t(language.get().as_str(), target_key),
        move || set_split(!split),
    );
}
