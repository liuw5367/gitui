//! 提交检查器（设计板 08 / 18 / 19）：Files / Info / Blame 三个页签。
//!
//! - Files：选中提交改动的文件（numstat，含 +/−），点一行看该文件的补丁；
//! - Info：提交元信息（SHA、tree、父提交、作者/提交者与时间、引用）与动作入口
//!   （当前只有复制 SHA 之外的入口标注未实现）；
//! - Blame：选中文件的逐行归属，同时列出该文件的历史。
//!
//! 宽度 320，与设计稿 `.insp` 一致。入口 `inspector` 收本功能的状态与用例；
//! 跨功能的动作（复制、编辑器打开、拣选/回滚）仍从 `Actions` 取。

use super::commands::HistoryCommands;
use super::state::HistoryState;
use crate::design;
use crate::i18n;
use crate::model::Tab;
use crate::Actions;
use nsh_core::prelude::*;

/// 面板宽度（设计板 `.insp`）。
pub(crate) const WIDTH: f32 = 320.0;

/// 检查器的页签（设计板 08 / 18 / 19）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InspectorTab {
    Files,
    Info,
    Blame,
}

/// 检查器不读筛选条件，`state` 只为与 `history` 统一入口签名；`use<>` 同 `history`，
/// 返回的元素不借用入参。
pub(crate) fn inspector(
    state: &HistoryState,
    cmd: &HistoryCommands,
    tab: Tab,
    language: Signal<SharedString>,
    selected_tab: Signal<InspectorTab>,
    actions: Actions,
) -> impl IntoElement + use<> {
    let _ = state;
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let select_file: std::sync::Arc<dyn Fn(usize)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |index: usize| cmd.select_file(index))
    };
    let commit = tab.selected_commit().cloned();
    let now = store::now_unix();
    let summary = commit
        .as_ref()
        .map(|commit| commit.summary.clone())
        .unwrap_or_default();
    let author = commit
        .as_ref()
        .map(|commit| commit.author.clone())
        .unwrap_or_default();
    let short = commit
        .as_ref()
        .map(|commit| commit.short.clone())
        .unwrap_or_default();
    let timestamp = commit
        .as_ref()
        .and_then(|commit| commit.date.parse::<i64>().ok())
        .unwrap_or(0);
    let tab_for_body = tab.clone();
    let actions_for_tab = actions.clone();
    // 页脚两个按钮作用于当前选中的文件（相对仓库根）。
    let selected_path = tab
        .selected_file
        .and_then(|index| tab.files.get(index))
        .map(|file| file.path.clone());
    return view()
        .w(px(WIDTH))
        .h_full()
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::panel())
        .border_l_1()
        .border_color(move || design::line())
        .child(
            view()
                .w_full()
                .px(px(12.0))
                .py(px(10.0))
                .flex_col()
                .gap(px(3.0))
                .border_b_1()
                .border_color(move || design::line())
                .child(
                    text(summary)
                        .text_size(px(crate::ui::BODY))
                        .font_weight(FontWeight(600))
                        .line_height(px(17.0))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(format!(
                        "{} · {} · {}",
                        author,
                        short,
                        crate::dates::format(language.get_untracked().as_str(), timestamp, now)
                    ))
                    .text_size(px(crate::ui::CODE))
                    
                    .line_height(px(13.0))
                    .text_color(move || design::text3()),
                )
                .child(
                    view()
                        .mt(px(2.0))
                        .h(px(18.0))
                        .px(px(5.0))
                        .items_center()
                        .rounded(px(3.0))
                        .bg(move || design::accent_soft())
                        .child(
                            text(move || {
                                i18n::with_arguments(
                                    language.get().as_str(),
                                    "files.atCommit",
                                    &[("short", short.clone())],
                                )
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .line_height(px(crate::ui::SECONDARY_LINE))
                            .text_color(move || design::accent()),
                        ),
                ),
        )
        .child(
            view()
                .w_full()
                .h(px(crate::ui::ROW))
                .px(px(crate::ui::GAP_LG))
                .child(crate::ui::tabs::tabs(
                    vec![
                        crate::ui::tabs::TabItem {
                            id: "files",
                            label: Box::new({
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.title")
                            }),
                        },
                        crate::ui::tabs::TabItem {
                            id: "info",
                            label: Box::new({
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.info")
                            }),
                        },
                        crate::ui::tabs::TabItem {
                            id: "blame",
                            label: Box::new({
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.blame")
                            }),
                        },
                    ],
                    match selected_tab.get_untracked() {
                        InspectorTab::Files => "files",
                        InspectorTab::Info => "info",
                        InspectorTab::Blame => "blame",
                    },
                    move |id| {
                        selected_tab.set(match id {
                            "info" => InspectorTab::Info,
                            "blame" => InspectorTab::Blame,
                            _ => InspectorTab::Files,
                        })
                    },
                )),
        )
        .child(dynamic(move || -> AnyElement {
            match selected_tab.get() {
                InspectorTab::Files => {
                    files_body(language, tab_for_body.clone(), select_file.clone()).into_element()
                }
                InspectorTab::Info => {
                    info_body(language, tab_for_body.clone(), actions_for_tab.clone()).into_element()
                }
                InspectorTab::Blame => blame_body(language, tab_for_body.clone()).into_element(),
            }
        }))
        .child(
            view()
                .w_full()
                .h(px(34.0))
                .px(px(12.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .border_t_1()
                .border_color(move || design::line())
                .child(crate::ui::button::button(
                           
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "files.copyPath")
                    },
                           crate::ui::button::ButtonVariant::Secondary,
                           crate::ui::button::ButtonSize::Small,
                           false,
                           {
                        let copy_text = actions.copy_text.clone();
                        let root = tab.path.clone();
                        let relative = selected_path.clone();
                        move || {
                            if let Some(relative) = relative.clone() {
                                copy_text(format!("{}/{}", root.trim_end_matches('/'), relative));
                            }
                        }
                    },
                       ))
                .child(view().flex_1())
                .child(crate::ui::button::button(
                           
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "files.openInEditor")
                    },
                           crate::ui::button::ButtonVariant::Secondary,
                           crate::ui::button::ButtonSize::Small,
                           false,
                           {
                        let open_editor = actions.open_editor.clone();
                        let root = tab.path.clone();
                        let relative = selected_path.clone();
                        move || {
                            if let Some(relative) = relative.clone() {
                                open_editor(format!("{}/{}", root.trim_end_matches('/'), relative));
                            }
                        }
                    },
                       )),
        );
}

fn files_body(
    language: Signal<SharedString>,
    tab: Tab,
    select_file: std::sync::Arc<dyn Fn(usize)>,
) -> AnyElement {
    let files = tab.files.clone();
    let selected = tab.selected_file;
    if files.is_empty() {
        return view()
            .w_full()
            .h(px(60.0))
            .items_center()
            .justify_center()
            .child(
                text(move || i18n::t(language.get().as_str(), "files.empty"))
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
            )
            .into_element();
    }
    // 表格：一列「文件」，行内容（图标 + 路径 + +/− 统计）由 render 回调自绘，
    // 行高 24 与设计板 08 的检查器行一致。
    let rows: Vec<Vec<String>> = files.iter().map(|file| vec![file.path.clone()]).collect();
    let files_for_render = files.clone();
    return crate::ui::table::table(crate::ui::table::TableSpec {
        columns: vec![crate::ui::table::Column {
            key: "file",
            title: None,
            width: None,
            fixed: false,
        }],
        rows,
        selected,
        row_height: Some(24.0),
        on_select: Box::new(move |index| select_file(index)),
        render: Some(Box::new(move |row, _key, _text| {
            let file = &files_for_render[row as usize];
            return view()
                .w_full()
                .h_full()
                .flex_row()
                .items_center()
                .gap(px(crate::ui::GAP_SM))
                .px(px(crate::ui::GAP_SM))
                .child(crate::ui::icon::icon_view("file", crate::ui::icon::IconSize::Compact, design::text3))
                .child(
                    text(file.path.clone())
                        .flex_1()
                        .min_w_0()
                        .text_size(px(crate::ui::CODE))
                        .line_height(px(crate::ui::CODE_LINE))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(format!("+{}", file.additions))
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::add_fg()),
                )
                .child(
                    text(format!("−{}", file.deletions))
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::del_fg()),
                )
                .into_element();
        })),
        on_row_press: None,
        on_context_menu: None,
    })
    .into_element();
}

fn info_body(language: Signal<SharedString>, tab: Tab, actions: Actions) -> AnyElement {
    let sha_for_actions = tab
        .selected_commit()
        .map(|commit| commit.sha.clone())
        .unwrap_or_default();
    let Some(details) = tab.details.clone() else {
        return view()
            .w_full()
            .h(px(40.0))
            .items_center()
            .justify_center()
            .child(
                text(move || i18n::t(language.get().as_str(), "files.empty"))
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
            )
            .into_element();
    };
    let now = store::now_unix();
    let mut rows: Vec<AnyElement> = Vec::new();
    let parents = if details.parents.is_empty() {
        "—".to_string()
    } else {
        details
            .parents
            .iter()
            .map(|parent| parent.chars().take(7).collect::<String>())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let refs = if details.refs.is_empty() {
        "—".to_string()
    } else {
        details.refs.join(", ")
    };
    for (key, value) in [
        ("files.info.sha", details.sha.clone()),
        ("files.info.tree", details.tree.chars().take(7).collect()),
        ("files.info.parents", parents),
        (
            "files.info.author",
            format!("{} <{}>", details.author, details.author_email),
        ),
        (
            "files.info.authored",
            crate::dates::format(language.get_untracked().as_str(), details.authored_at, now),
        ),
        (
            "files.info.committer",
            format!("{} <{}>", details.committer, details.committer_email),
        ),
        (
            "files.info.committed",
            crate::dates::format(language.get_untracked().as_str(), details.committed_at, now),
        ),
        ("files.info.refs", refs),
    ] {
        rows.push(
            view()
                .w_full()
                .py(px(3.0))
                .flex_col()
                .gap(px(1.0))
                .child(
                    text(move || i18n::t(language.get_untracked().as_str(), key))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(value)
                        .text_size(px(crate::ui::CODE))
                        
                        .line_height(px(14.0))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .into_element(),
        );
    }
    return scroll()
        .w_full()
        .h_full()
        .child(
            view()
                .w_full()
                .px(px(12.0))
                .py(px(8.0))
                .flex_col()
                .child(rows)
                .child(
                    view()
                        .w_full()
                        .mt(px(6.0))
                        .pt(px(6.0))
                        .flex_wrap()
                        .gap(px(6.0))
                        .border_t_1()
                        .border_color(move || design::line())
                        .child(crate::ui::button::button(
                                   
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.copySha")
                            },
                                   crate::ui::button::ButtonVariant::Secondary,
                                   crate::ui::button::ButtonSize::Small,
                                   false,
                                   {
                                let copy_text = actions.copy_text.clone();
                                let sha = sha_for_actions.clone();
                                move || copy_text(sha.clone())
                            },
                               ))
                        .child(crate::ui::button::button(
                                   
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.cherryPick")
                            },
                                   crate::ui::button::ButtonVariant::Secondary,
                                   crate::ui::button::ButtonSize::Small,
                                   false,
                                   {
                                let cherry_pick = actions.cherry_pick.clone();
                                let sha = sha_for_actions.clone();
                                move || cherry_pick(sha.clone())
                            },
                               ))
                        .child(crate::ui::button::button(
                                   
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.revert")
                            },
                                   crate::ui::button::ButtonVariant::Secondary,
                                   crate::ui::button::ButtonSize::Small,
                                   false,
                                   {
                                let revert_commit = actions.revert_commit.clone();
                                let sha = sha_for_actions.clone();
                                move || revert_commit(sha.clone())
                            },
                               )),
                ),
        )
        .into_element();
}


fn blame_body(language: Signal<SharedString>, tab: Tab) -> impl IntoElement {
    let now = store::now_unix();
    let lines = tab.blame.clone();
    let history = tab.file_history.clone();
    let mut rows: Vec<AnyElement> = Vec::new();
    for line in &lines {
        let short = line.short.clone();
        let author = line.author.clone();
        let timestamp = line.timestamp;
        let number = line.line;
        let text_value = line.text.clone();
        rows.push(
            view()
                .w_full()
                .h(px(18.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(short)
                        .w(px(52.0))
                        .text_size(px(crate::ui::CODE))
                        
                        .line_height(px(12.0))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(number.to_string())
                        .w(px(26.0))
                        .text_size(px(crate::ui::CODE))
                        
                        .line_height(px(12.0))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(text_value)
                        .flex_1()
                        .text_size(px(crate::ui::CODE))
                        
                        .line_height(px(13.0))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Clip),
                )
                .into_element(),
        );
        let _ = (author, timestamp);
    }
    let mut history_rows: Vec<AnyElement> = Vec::new();
    for commit in history.iter().take(12) {
        let summary = commit.summary.clone();
        let short = commit.short.clone();
        let timestamp = commit.date.parse::<i64>().unwrap_or(0);
        history_rows.push(
            view()
                .w_full()
                .h(px(20.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(short)
                        .text_size(px(crate::ui::CODE))
                        
                        .line_height(px(12.0))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(summary)
                        .flex_1()
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(move || {
                        crate::dates::format(language.get().as_str(), timestamp, now)
                    })
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
                )
                .into_element(),
        );
    }
    if lines.is_empty() {
        return scroll().w_full().h_full().child(
            view()
                .w_full()
                .h(px(40.0))
                .items_center()
                .justify_center()
                .child(
                    text(move || i18n::t(language.get().as_str(), "files.blameHint"))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                ),
        );
    }
    return scroll().w_full().h_full().child(
        view()
            .w_full()
            .px(px(6.0))
            .py(px(4.0))
            .flex_col()
            .child(rows)
            .child(
                view()
                    .w_full()
                    .mt(px(6.0))
                    .pt(px(4.0))
                    .border_t_1()
                    .border_color(move || design::line())
                    .child(
                        text(move || i18n::t(language.get().as_str(), "files.history"))
                            .text_size(px(crate::ui::SECONDARY))
                            .font_weight(FontWeight(600))
                            .line_height(px(13.0))
                            .text_color(move || design::text3()),
                    )
                    .child(history_rows),
            ),
    );
}
