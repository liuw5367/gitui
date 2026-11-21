//! 表单对话框（设计板 22）：新建分支 / 标签 / 远程 / 贮藏。
//!
//! `alert_dialog` 只有按钮没有输入框，这里自建一个居中的模态浮层，带 1–2 个输入
//! 字段与取消/确认。确认后由调用方执行真正的 Git 操作。

use crate::design::{self, metrics};
use crate::i18n;
use nsh_core::prelude::*;
use std::collections::BTreeSet;

/// 对话框的种类，决定标题、字段与确认文案。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormKind {
    NewBranch,
    NewTag,
    AddRemote,
    NewStash,
    StashSelectedFiles,
    RenameBranch,
    EditRemote,
    Identity,
    NewWorktree,
}

impl FormKind {
    fn title(self) -> &'static str {
        return match self {
            FormKind::NewBranch => "form.newBranch",
            FormKind::NewTag => "form.newTag",
            FormKind::AddRemote => "form.addRemote",
            FormKind::NewStash => "form.newStash",
            FormKind::StashSelectedFiles => "stash.selectedTitle",
            FormKind::RenameBranch => "form.renameBranch",
            FormKind::EditRemote => "form.editRemote",
            FormKind::Identity => "form.identity",
            FormKind::NewWorktree => "worktree.newTitle",
        };
    }

    /// 第一个字段的标签。
    fn first_label(self) -> &'static str {
        return match self {
            FormKind::NewBranch | FormKind::NewTag => "form.name",
            FormKind::AddRemote => "form.remoteName",
            FormKind::NewStash | FormKind::StashSelectedFiles => "form.message",
            FormKind::RenameBranch => "form.newName",
            FormKind::EditRemote => "form.urlNew",
            FormKind::Identity => "form.authorName",
            FormKind::NewWorktree => "worktree.path",
        };
    }

    /// 第三个字段的标签（只有附注标签需要说明）。
    fn third_label(self) -> Option<&'static str> {
        return match self {
            FormKind::NewTag => Some("form.message"),
            FormKind::NewWorktree => Some("worktree.startPoint"),
            _ => None,
        };
    }

    /// 复选项的标签；`None` 表示没有复选。
    fn flag_label(self) -> Option<&'static str> {
        return match self {
            FormKind::NewBranch => Some("form.checkoutAfter"),
            FormKind::NewTag => Some("form.annotated"),
            FormKind::NewWorktree => Some("worktree.createBranch"),
            FormKind::NewStash | FormKind::StashSelectedFiles => Some("form.includeUntracked"),
            _ => None,
        };
    }

    /// 复选的初始值：贮藏默认带上未跟踪文件，其它默认不勾。
    fn flag_default(self) -> bool {
        return matches!(self, FormKind::NewStash | FormKind::StashSelectedFiles);
    }

    /// 第二个字段的标签；`None` 表示只有一个字段。
    fn second_label(self) -> Option<&'static str> {
        return match self {
            FormKind::NewBranch | FormKind::NewTag => Some("form.from"),
            FormKind::NewWorktree => Some("worktree.branch"),
            FormKind::AddRemote => Some("form.url"),
            FormKind::NewStash | FormKind::StashSelectedFiles => None,
            FormKind::RenameBranch | FormKind::EditRemote => None,
            FormKind::Identity => Some("form.authorEmail"),
        };
    }

    fn confirm(self) -> &'static str {
        return match self {
            FormKind::NewBranch => "form.create",
            FormKind::NewTag => "form.create",
            FormKind::AddRemote => "form.add",
            FormKind::NewStash => "form.stash",
            FormKind::StashSelectedFiles => "stash.stashSelectedFiles",
            FormKind::RenameBranch => "form.rename",
            FormKind::EditRemote => "form.save",
            FormKind::Identity => "form.save",
            FormKind::NewWorktree => "worktree.create",
        };
    }
}

/// 对话框状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FormState {
    pub kind: FormKind,
    pub target_tab_id: Option<u64>,
    pub first: String,
    pub second: String,
    /// 第三个字段（目前只有附注标签的说明用）。
    pub third: String,
    /// 复选项的值；含义由 [`FormKind::flag_label`] 决定。
    pub flag: bool,
    /// 远端分支菜单要求建立上游跟踪关系。
    pub track_upstream: bool,
    /// Paths shown by the selected-file stash picker and its selection state.
    pub stash_paths: Vec<(String, bool)>,
    pub stash_selected_paths: BTreeSet<String>,
    pub error: Option<String>,
}

impl FormState {
    pub fn new(kind: FormKind) -> FormState {
        return FormState {
            kind,
            target_tab_id: None,
            first: String::new(),
            second: String::new(),
            third: String::new(),
            flag: kind.flag_default(),
            track_upstream: false,
            stash_paths: Vec::new(),
            stash_selected_paths: BTreeSet::new(),
            error: None,
        };
    }

    pub(crate) fn selected_stash_paths(&self) -> Vec<String> {
        return self
            .stash_paths
            .iter()
            .filter(|(path, untracked)| {
                self.stash_selected_paths.contains(path) && (!untracked || self.flag)
            })
            .map(|(path, _)| path.clone())
            .collect();
    }
}

/// 模态表单。`state` 为 `None` 时什么都不画。
pub(crate) fn form_dialog(
    language: Signal<SharedString>,
    state: Signal<Option<FormState>>,
    on_submit: std::sync::Arc<dyn Fn(FormState)>,
    on_cancel: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
    let dismiss = on_cancel.clone();
    return crate::ui::dialog::modal(
        move || state.get().is_some(),
        move || {
            state
                .get()
                .map(|form| i18n::t(language.get().as_str(), form.kind.title()))
                .unwrap_or_default()
        },
        120,
        move || {
            let state = state;
            let language = language;
            let on_submit = on_submit.clone();
            let on_cancel = on_cancel.clone();
            dynamic(move || -> AnyElement {
                let Some(form) = state.get() else {
                    return view().w(px(0.0)).h(px(0.0)).into_element();
                };
                let submit_form = form.clone();
                let on_submit = on_submit.clone();
                let on_cancel = on_cancel.clone();
                let form_for_field_one = form.clone();
                let form_for_field_two = form.clone();
                let form_for_third = form.clone();
                let form_for_flag = form.clone();
                let form_for_stash_paths = form.clone();
                let form_for_worktree_path = form.clone();
                return crate::ui::dialog::modality(
                    view()
                        .w(px(420.0))
                        .p(px(16.0))
                        .flex_col()
                        .gap(px(10.0))
                        .rounded(px(metrics::RADIUS_PANEL))
                        .border_1()
                        .border_color(move || design::line())
                        .bg(move || design::token("popover"))
                        .child(
                            text(move || i18n::t(language.get().as_str(), form.kind.title()))
                                .text_size(px(crate::ui::TITLE))
                                .font_weight(FontWeight(600))
                                .line_height(px(18.0))
                                .text_color(move || design::text()),
                        )
                        .child(field(
                            language,
                            form.kind.first_label(),
                            form_for_field_one.clone(),
                            |state: &FormState| state.first.clone(),
                            {
                                let form = form_for_field_one.clone();
                                move |text: String| {
                                    let mut next = form.clone();
                                    next.first = text;
                                    state.set(Some(next));
                                }
                            },
                        ))
                        .child(dynamic(move || -> AnyElement {
                            if form.kind != FormKind::NewWorktree {
                                return view().w_full().h(px(0.0)).into_element();
                            }
                            return choose_worktree_path(
                                language,
                                form_for_worktree_path.clone(),
                                state,
                            )
                            .into_element();
                        }))
                        .child(dynamic(move || -> AnyElement {
                            let Some(label_key) = form.kind.second_label() else {
                                return view().w_full().h(px(0.0)).into_element();
                            };
                            return field(
                                language,
                                label_key,
                                form_for_field_two.clone(),
                                |state: &FormState| state.second.clone(),
                                {
                                    let state = state;
                                    let form = form_for_field_two.clone();
                                    move |text: String| {
                                        let mut next = form.clone();
                                        next.second = text;
                                        state.set(Some(next));
                                    }
                                },
                            )
                            .into_element();
                        }))
                        .child(dynamic(move || -> AnyElement {
                            if form.kind != FormKind::StashSelectedFiles {
                                return view().w_full().h(px(0.0)).into_element();
                            }
                            return stash_file_picker(
                                language,
                                form_for_stash_paths.clone(),
                                state,
                            )
                            .into_element();
                        }))
                        .child(dynamic(move || -> AnyElement {
                            let Some(label_key) = form.kind.third_label() else {
                                return view().w_full().h(px(0.0)).into_element();
                            };
                            if form.kind == FormKind::NewWorktree && !form.flag {
                                return view().w_full().h(px(0.0)).into_element();
                            }
                            let form_for_third = form_for_third.clone();
                            return field(
                                language,
                                label_key,
                                form_for_third.clone(),
                                |state: &FormState| state.third.clone(),
                                {
                                    let state = state;
                                    let form = form_for_third.clone();
                                    move |text: String| {
                                        let mut next = form.clone();
                                        next.third = text;
                                        state.set(Some(next));
                                    }
                                },
                            )
                            .into_element();
                        }))
                        .child(dynamic(move || -> AnyElement {
                            let Some(label_key) = form.kind.flag_label() else {
                                return view().w_full().h(px(0.0)).into_element();
                            };
                            if form.track_upstream {
                                return view().w_full().h(px(0.0)).into_element();
                            }
                            let form_for_flag = form_for_flag.clone();
                            return flag_row(language, label_key, form_for_flag, state)
                                .into_element();
                        }))
                        .child(
                            view()
                                .w_full()
                                .h(move || {
                                    if state
                                        .get()
                                        .map(|form| form.error.is_some())
                                        .unwrap_or(false)
                                    {
                                        px(16.0)
                                    } else {
                                        px(0.0)
                                    }
                                })
                                .overflow_hidden()
                                .items_center()
                                .child(
                                    text(move || {
                                        state.get().and_then(|form| form.error).unwrap_or_default()
                                    })
                                    .text_size(px(crate::ui::SECONDARY))
                                    .line_height(px(crate::ui::SECONDARY_LINE))
                                    .text_color(move || design::danger()),
                                ),
                        )
                        .child(
                            view()
                                .w_full()
                                .flex_row()
                                .items_center()
                                .gap(px(8.0))
                                .child(view().flex_1())
                                .child(crate::ui::button::button(
                                    move || i18n::t(language.get().as_str(), "common.cancel"),
                                    crate::ui::button::ButtonVariant::Secondary,
                                    crate::ui::button::ButtonSize::Regular,
                                    false,
                                    {
                                        let on_cancel = on_cancel.clone();
                                        move || on_cancel()
                                    },
                                ))
                                .child(crate::ui::button::button(
                                    move || i18n::t(language.get().as_str(), form.kind.confirm()),
                                    crate::ui::button::ButtonVariant::Primary,
                                    crate::ui::button::ButtonSize::Regular,
                                    form.kind == FormKind::StashSelectedFiles
                                        && form.selected_stash_paths().is_empty(),
                                    {
                                        let on_submit = on_submit.clone();
                                        move || on_submit(submit_form.clone())
                                    },
                                )),
                        )
                        .into_element(),
                )
                .into_element();
            })
        },
        move || dismiss(),
    );
}

fn choose_worktree_path(
    language: Signal<SharedString>,
    form: FormState,
    state: Signal<Option<FormState>>,
) -> impl IntoElement {
    let choose = crate::ui::button::button(
        move || i18n::t(language.get().as_str(), "worktree.choosePath"),
        crate::ui::button::ButtonVariant::Secondary,
        crate::ui::button::ButtonSize::Small,
        false,
        move || {
            let answer = pick_paths(nsh_core::file_dialog::OpenDialog::folders());
            let state = state;
            let form = form.clone();
            spawn(async move {
                let Ok(Some(paths)) = answer.await else {
                    return;
                };
                let Some(path) = paths.first() else {
                    return;
                };
                let mut updated = form;
                updated.first = path.display().to_string();
                updated.error = None;
                state.set(Some(updated));
            });
        },
    );
    return view().w_full().flex_row().justify_end().child(choose);
}

fn stash_file_picker(
    language: Signal<SharedString>,
    form: FormState,
    state: Signal<Option<FormState>>,
) -> impl IntoElement {
    let selected = form.selected_stash_paths();
    let mut rows = Vec::new();
    for (path, untracked) in form.stash_paths.iter() {
        let path_for_action = path.clone();
        let label = if *untracked {
            i18n::with_arguments(
                language.get_untracked().as_str(),
                "stash.untrackedFile",
                &[("path", path.clone())],
            )
        } else {
            path.clone()
        };
        let checked = form.stash_selected_paths.contains(path);
        let disabled = *untracked && !form.flag;
        let form_for_toggle = form.clone();
        rows.push(crate::ui::check::check_row(
            move || label.clone(),
            checked,
            disabled,
            move |next| {
                let mut updated = form_for_toggle.clone();
                if next {
                    updated.stash_selected_paths.insert(path_for_action.clone());
                } else {
                    updated
                        .stash_selected_paths
                        .remove(path_for_action.as_str());
                }
                state.set(Some(updated));
            },
        ));
    }
    let form_for_all = form.clone();
    let can_select_all = form
        .stash_paths
        .iter()
        .any(|(_, untracked)| !*untracked || form.flag);
    let select_all = crate::ui::button::button(
        move || i18n::t(language.get().as_str(), "stash.selectAll"),
        crate::ui::button::ButtonVariant::Ghost,
        crate::ui::button::ButtonSize::Small,
        !can_select_all,
        move || {
            let mut updated = form_for_all.clone();
            updated.stash_selected_paths = updated
                .stash_paths
                .iter()
                .filter(|(_, untracked)| !*untracked || updated.flag)
                .map(|(path, _)| path.clone())
                .collect();
            state.set(Some(updated));
        },
    );
    let form_for_clear = form.clone();
    let clear = crate::ui::button::button(
        move || i18n::t(language.get().as_str(), "stash.clearSelection"),
        crate::ui::button::ButtonVariant::Ghost,
        crate::ui::button::ButtonSize::Small,
        selected.is_empty(),
        move || {
            let mut updated = form_for_clear.clone();
            updated.stash_selected_paths.clear();
            state.set(Some(updated));
        },
    );
    let picker_content = if rows.is_empty() {
        view()
            .w_full()
            .h(px(42.0))
            .items_center()
            .justify_center()
            .child(
                text(move || i18n::t(language.get().as_str(), "stash.noChangedFiles"))
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
            )
            .into_element()
    } else {
        scroll()
            .w_full()
            .max_h(px(220.0))
            .child(view().w_full().flex_col().child(rows))
            .into_element()
    };
    return view()
        .w_full()
        .flex_col()
        .gap(px(4.0))
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .child(
                    text(move || {
                        i18n::with_arguments(
                            language.get().as_str(),
                            "stash.selectFiles",
                            &[("count", selected.len().to_string())],
                        )
                    })
                    .flex_1()
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text2()),
                )
                .child(select_all)
                .child(clear),
        )
        .child(picker_content);
}

/// 一行复选：走 `ui::check::check_inline`（原生 Checkbox + 统一指示器）。
fn flag_row(
    language: Signal<SharedString>,
    label_key: &'static str,
    form: FormState,
    state: Signal<Option<FormState>>,
) -> impl IntoElement {
    let toggle = {
        let form = form.clone();
        move |next: bool| {
            let mut updated = form.clone();
            updated.flag = next;
            state.set(Some(updated));
        }
    };
    return view().w_full().h(px(22.0)).flex_row().items_center().child(
        crate::ui::check::check_inline(
            move || i18n::t(language.get().as_str(), label_key),
            form.flag,
            22.0,
            14.0,
            false,
            toggle,
        ),
    );
}

fn field(
    language: Signal<SharedString>,
    label_key: &'static str,
    form: FormState,
    read: impl Fn(&FormState) -> String + 'static,
    write: impl Fn(String) + 'static,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(4.0))
        .child(
            text(move || i18n::t(language.get().as_str(), label_key))
                .text_size(px(crate::ui::SECONDARY))
                .font_weight(FontWeight(600))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text2()),
        )
        .child(crate::ui::text_input::text_input_bound(
            {
                let form = form.clone();
                move || read(&form)
            },
            || String::new(),
            false,
            move |text: String| write(text),
        ));
}
