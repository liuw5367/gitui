//! diff 视图与它的显示开关：顶部工具栏（导出、复制、布局/视图开关）+ 正文。
//!
//! 正文自 P2 起交给 `ui::diff_surface`（包装 `nsh_base::DiffView`），等宽 12 / 行高
//! 18、行号与正负行配色都由它负责。本文件只剩工具栏与开关。
//!
//! 补丁在 `git` 边界就解析成 `domain::diff::Patch`，这里只负责展示与开关。
//! 选项与开关动作放在 `shared`：历史页与变更页共用同一份显示偏好。

use crate::design::{self, metrics};
use crate::features::SessionHandle;
use crate::i18n;
use domain::diff::Patch;
use nsh_core::prelude::*;

/// diff 的显示开关：布局、上下文折叠/展开、换行与空白处理。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DiffOptions {
    /// 折叠未改动的大段上下文。
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

/// diff 的用例：切换视图与显示开关、文件树的两项开关、导出当前补丁。
#[derive(Clone)]
pub(crate) struct DiffCommands {
    pub(crate) state: DiffState,
    /// 文件树两项开关写进设置，因此需要设置与它所在目录。
    pub(crate) settings: Signal<store::Settings>,
    pub(crate) store_dir: std::path::PathBuf,
    /// 导出补丁要读当前标签页、写状态栏提示；这三样都在会话句柄里。
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

    /// 导出当前显示的原始补丁到用户选择的路径（原 `Actions::export_patch`）。
    ///
    /// 没有选择路径就不写，不落默认文件名；补丁为空时只提示，不弹保存框。
    pub(crate) fn export_patch(&self, patch: String) {
        let notice = self.session.notice;
        let language = self.session.language;
        if patch.trim().is_empty() {
            let lang = language.get_untracked();
            crate::interactions::notify(notice, i18n::t(lang.as_str(), "diff.nothingToExport"));
            return;
        }
        let lang = language.get_untracked();
        let suggested = i18n::t(lang.as_str(), "diff.exportName");
        let pending = pick_save_path(nsh_core::file_dialog::SaveDialog::new().file_name(suggested));
        spawn(async move {
            let Ok(Some(path)) = pending.await else {
                return;
            };
            match std::fs::write(&path, patch.as_bytes()) {
                Ok(()) => {
                    crate::interactions::notify(notice, i18n::t(lang.as_str(), "diff.exported"));
                }
                Err(error) => {
                    crate::interactions::notify(notice, error.to_string());
                }
            }
        });
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn diff(
    state: &DiffState,
    cmd: &DiffCommands,
    language: Signal<SharedString>,
    label: String,
    patch: Patch,
    raw_patch: Patch,
    empty_feedback: Option<&'static str>,
) -> impl IntoElement {
    // 只改展示文本；复制、导出与统计都取未转换的原始补丁。
    let options = state.options.get();
    let ignore_whitespace = state.ignore_whitespace.get();
    let patch_for_view = if options.whitespace {
        show_whitespace(&patch.raw)
    } else {
        patch.raw.clone()
    };
    let additions: u32 = raw_patch.files.iter().map(|file| file.additions).sum();
    let deletions: u32 = raw_patch.files.iter().map(|file| file.deletions).sum();
    let hunk_count = patch
        .lines
        .iter()
        .filter(|line| line.kind == domain::diff::DiffLineKind::Hunk)
        .count() as u32;
    let hunk_position = signal(if hunk_count > 0 { Some(1_u32) } else { None });
    let diff_handle = crate::ui::diff_surface::DiffHandle::new();
    let file_label = raw_patch
        .files
        .first()
        .map(|file| file.path.clone())
        .unwrap_or_default();
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let split_view = state.split.get();
    let toggle_option: std::sync::Arc<dyn Fn(&'static str)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |which: &'static str| cmd.toggle_diff_option(which))
    };
    let set_split_mode: std::sync::Arc<dyn Fn(bool)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |split: bool| cmd.set_diff_split(split))
    };
    let copy_patch = {
        let cmd = cmd.clone();
        let notice = cmd.session.notice;
        move |text: String| crate::shared::os::copy_text(notice, language, text)
    };
    let hunk_patches = raw_hunks(&patch.raw);
    let copy_hunk: std::sync::Arc<dyn Fn(&nsh_base::DiffHunk)> = {
        let notice = cmd.session.notice;
        std::sync::Arc::new(move |hunk: &nsh_base::DiffHunk| {
            if let Some(patch) = hunk_patches.get(hunk.index as usize) {
                crate::shared::os::copy_text(notice, language, patch.clone());
            }
        })
    };
    let export_patch = {
        let cmd = cmd.clone();
        let patch = raw_patch.raw.clone();
        move || cmd.export_patch(patch.clone())
    };
    let copy_text = raw_patch.raw.clone();
    let previous_hunk = {
        let handle = diff_handle.clone();
        let position = hunk_position;
        move || {
            if let Some(index) = handle.previous_hunk() {
                position.set(Some(index + 1));
            }
        }
    };
    let next_hunk = {
        let handle = diff_handle.clone();
        let position = hunk_position;
        move || {
            if let Some(index) = handle.next_hunk() {
                position.set(Some(index + 1));
            }
        }
    };
    return view()
        .w_full()
        .flex_1()
        .min_h_0()
        .flex_col()
        .bg(move || design::content())
        .child(
            view()
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
                    text(file_label)
                        .text_size(px(crate::ui::CODE))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(label)
                        .text_size(px(crate::ui::CODE))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(format!("+{} −{}", additions, deletions))
                        .text_size(px(crate::ui::CODE))
                        .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(fold_toggle(language, options, toggle_option.clone()))
                .child(wrap_toggle(language, options, toggle_option.clone()))
                .child(space_toggle(language, options, toggle_option.clone()))
                .child(ignore_whitespace_toggle(
                    language,
                    ignore_whitespace,
                    toggle_option.clone(),
                ))
                .child(
                    text(move || {
                        let position = hunk_position.get().unwrap_or(0);
                        format!("{} / {}", position, hunk_count)
                    })
                    .text_size(px(crate::ui::CODE))
                    .text_color(move || design::text3()),
                )
                .child(crate::ui::button::icon_button(
                    "chev-left",
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "diff.previousChange")
                    },
                    crate::ui::button::ButtonSize::Small,
                    hunk_count == 0,
                    move || previous_hunk(),
                ))
                .child(crate::ui::button::icon_button(
                    "chev-right",
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "diff.nextChange")
                    },
                    crate::ui::button::ButtonSize::Small,
                    hunk_count == 0,
                    move || next_hunk(),
                ))
                .child(crate::ui::button::button(
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "diff.export")
                    },
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || export_patch(),
                ))
                .child(mode_toggle(language, split_view, set_split_mode.clone()))
                .child(crate::ui::button::icon_button(
                    "copy",
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "diff.copy")
                    },
                    crate::ui::button::ButtonSize::Small,
                    copy_text.trim().is_empty(),
                    move || copy_patch(copy_text.clone()),
                )),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(dynamic(move || -> AnyElement {
                    if raw_patch.raw.trim().is_empty() {
                        if let Some(key) = empty_feedback {
                            return feedback_state(language, key).into_element();
                        }
                        return empty_state(language).into_element();
                    }
                    if patch.raw.trim().is_empty() || hunk_count == 0 {
                        return no_text_changes(language).into_element();
                    }
                    // 渲染交给 nsh 的 DiffView：分栏/统一、折叠、软换行、行号与
                    // 指示条都由它提供，这里只传 patch 与两个显示开关。
                    let layout = if split_view {
                        crate::ui::diff_surface::Layout::Split
                    } else {
                        crate::ui::diff_surface::Layout::Unified
                    };
                    return crate::ui::diff_surface::diff_surface(
                        SharedString::from(patch_for_view.clone()),
                        layout,
                        options.wrap,
                        !options.fold,
                        &diff_handle,
                        language,
                        copy_hunk.clone(),
                    )
                    .into_element();
                })),
        );
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
    return crate::ui::button::icon_button_tuned(
        name,
        move || i18n::t(language.get().as_str(), label_key),
        crate::ui::button::ButtonSize::Small,
        Some(crate::ui::icon::IconSize::Compact),
        Some(foreground),
        false,
        move || toggle(hint_key),
    )
    .bg(move || {
        if active {
            design::accent_soft()
        } else {
            AdaptiveColor::TRANSPARENT
        }
    });
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
    return view()
        .h(px(20.0))
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .p(px(2.0))
        .rounded(px(5.0))
        .border_1()
        .border_color(move || design::line_soft())
        .bg(move || design::panel2())
        .child(mode_button(language, "history.unified", !split, {
            let set_split = set_split.clone();
            move || set_split(false)
        }))
        .child(mode_button(language, "history.split", split, move || {
            set_split(true)
        }));
}

fn mode_button(
    language: Signal<SharedString>,
    key: &'static str,
    active: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    // Unified / Split 二选一：选中的用 Secondary 表达，未选中用 Ghost。
    let variant = if active {
        crate::ui::button::ButtonVariant::Secondary
    } else {
        crate::ui::button::ButtonVariant::Ghost
    };
    return crate::ui::button::button(
        move || i18n::t(language.get().as_str(), key),
        variant,
        crate::ui::button::ButtonSize::Small,
        false,
        on_press,
    );
}
