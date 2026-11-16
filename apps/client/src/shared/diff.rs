//! diff 视图与它的显示开关：顶部工具栏（导出、复制、三个显示开关）+ 正文。
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

/// diff 的三个显示开关（设计板 04 的 fold / wrap / whitespace）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DiffOptions {
    /// 折叠未改动的大段上下文。
    pub fold: bool,
    /// 长行换行而不是横向截断。
    pub wrap: bool,
    /// 把行尾空格与制表符显示成可见符号。
    pub whitespace: bool,
}

impl DiffOptions {
}

/// diff 的显示状态：并排/合并、三个显示开关。
#[derive(Clone, Copy)]
pub(crate) struct DiffState {
    /// 并排（split）还是合并（unified）视图。
    pub(crate) split: Signal<bool>,
    pub(crate) options: Signal<DiffOptions>,
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
            _ => return,
        }
        self.state.options.set(next);
    }

    /// 导出当前补丁到用户选择的路径（原 `Actions::export_patch`）。
    ///
    /// 没有选择路径就不写，不落默认文件名；补丁为空时只提示，不弹保存框。
    pub(crate) fn export_patch(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let notice = self.session.notice;
        let language = self.session.language;
        let list = tabs.get_untracked();
        let Some(tab) = list.get(active.get_untracked()) else {
            return;
        };
        let patch = tab.patch.raw.clone();
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
) -> impl IntoElement {
    // 渲染与复制都要字节一致的原文，直接取解析时留下的 raw。
    let patch_for_view = patch.raw.clone();
    let additions: u32 = patch.files.iter().map(|file| file.additions).sum();
    let deletions: u32 = patch.files.iter().map(|file| file.deletions).sum();
    let file_label = patch
        .files
        .first()
        .map(|file| file.path.clone())
        .unwrap_or_default();
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let options = state.options.get();
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
    let export_patch = {
        let cmd = cmd.clone();
        move || cmd.export_patch()
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
                .child(crate::ui::icon::icon_view("diff", crate::ui::icon::IconSize::Button, design::text3))
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
                    move || "Copy".to_string(),
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || copy_patch(patch.raw.clone()),
                )),
        )
        .child(
            view().w_full().flex_1().min_h_0().child(
                dynamic(move || -> AnyElement {
                    if patch.lines.is_empty() {
                        return empty_state(language).into_element();
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
                        None,
                    )
                    .into_element();
                }),
            ),
        );
}

/// 一个图标开关（fold / wrap / whitespace），开启时用强调色。
fn diff_toggle(
    name: &'static str,
    hint_key: &'static str,
    active: bool,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    return view()
        .h(px(20.0))
        .w(px(22.0))
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .bg(move || {
            if active {
                design::accent_soft()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .hover(|style| style.bg(move || design::hover()))
        .on_press(move || toggle(hint_key))
        .child(crate::ui::icon::icon_view(
            name,
            crate::ui::icon::IconSize::Compact,
            if active {
                design::accent
            } else {
                design::text3
            },
        ));
}

fn fold_toggle(
    language: Signal<SharedString>,
    options: DiffOptions,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    let _ = language;
    return diff_toggle("fold", "fold", options.fold, toggle);
}

fn wrap_toggle(
    language: Signal<SharedString>,
    options: DiffOptions,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    let _ = language;
    return diff_toggle("wrap", "wrap", options.wrap, toggle);
}

fn space_toggle(
    language: Signal<SharedString>,
    options: DiffOptions,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    let _ = language;
    return diff_toggle("space", "whitespace", options.whitespace, toggle);
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
        .child(mode_button(
            language,
            "history.unified",
            !split,
            {
                let set_split = set_split.clone();
                move || set_split(false)
            },
        ))
        .child(mode_button(language, "history.split", split, move || set_split(true)));
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
