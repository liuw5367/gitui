//! 表单对话框（设计板 22）：新建分支 / 标签 / 远程 / 贮藏。
//!
//! `alert_dialog` 只有按钮没有输入框，这里自建一个居中的模态浮层，带 1–2 个输入
//! 字段与取消/确认。确认后由调用方执行真正的 Git 操作。

use crate::design;
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

    fn placeholder(self, label: &str) -> &'static str {
        match label {
            "form.from" | "worktree.startPoint" => "form.placeholders.revision",
            "form.url" | "form.urlNew" => "form.placeholders.remoteUrl",
            "form.remoteName" => "form.placeholders.remoteName",
            "form.authorName" => "form.placeholders.authorName",
            "form.authorEmail" => "form.placeholders.authorEmail",
            "worktree.path" => "form.placeholders.worktreePath",
            "form.message" if matches!(self, Self::NewStash | Self::StashSelectedFiles) => {
                "form.placeholders.stashMessage"
            }
            "form.message" => "form.placeholders.commitMessage",
            _ if self == Self::NewTag => "form.placeholders.tagName",
            _ => "form.placeholders.branchName",
        }
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
    crate::ui::dialog::modal(
        move || state.get().is_some(),
        move || {
            state
                .get()
                .map(|form| i18n::t(language.get().as_str(), form.kind.title()))
                .unwrap_or_default()
        },
        120,
        move || {
            let on_submit = on_submit.clone();
            let on_cancel = on_cancel.clone();
            // 文本编辑只更新字段绑定；只有字段结构改变时才重建面板。
            let structure = computed(move || {
                state.with(|state| {
                    state.as_ref().map(|form| {
                        (
                            form.kind,
                            form.track_upstream,
                            form.kind == FormKind::NewWorktree && form.flag,
                        )
                    })
                })
            });
            dynamic(move || {
                let Some((kind, track_upstream, worktree_branch)) = structure.get() else {
                    return view().into_element();
                };
                let mut fields = vec![
                    field(
                        language,
                        kind.first_label(),
                        kind.placeholder(kind.first_label()),
                        state,
                        |form| form.first.clone(),
                        |form, value| form.first = value,
                    )
                    .into_element(),
                ];
                if kind == FormKind::NewWorktree {
                    fields.push(
                        dynamic(move || {
                            choose_worktree_path(language, state.get().unwrap(), state)
                        })
                        .into_element(),
                    );
                }
                if let Some(label) = kind.second_label() {
                    fields.push(
                        field(
                            language,
                            label,
                            kind.placeholder(label),
                            state,
                            |form| form.second.clone(),
                            |form, value| form.second = value,
                        )
                        .into_element(),
                    );
                }
                if kind == FormKind::StashSelectedFiles {
                    fields.push(
                        dynamic(move || stash_file_picker(language, state.get().unwrap(), state))
                            .into_element(),
                    );
                }
                if let Some(label) = kind.third_label() {
                    if kind != FormKind::NewWorktree || worktree_branch {
                        fields.push(
                            field(
                                language,
                                label,
                                kind.placeholder(label),
                                state,
                                |form| form.third.clone(),
                                |form, value| form.third = value,
                            )
                            .into_element(),
                        );
                    }
                }
                if let Some(label) = kind.flag_label() {
                    if !track_upstream {
                        fields.push(
                            dynamic(move || flag_row(language, label, state.get().unwrap(), state))
                                .into_element(),
                        );
                    }
                }
                fields.push(
                    when(
                        move || {
                            state
                                .with(|form| form.as_ref().is_some_and(|form| form.error.is_some()))
                        },
                        move || {
                            text(move || {
                                state.get().and_then(|form| form.error).unwrap_or_default()
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .line_height(px(crate::ui::SECONDARY_LINE))
                            .text_color(design::danger)
                        },
                    )
                    .into_element(),
                );
                let body = view()
                    .w_full()
                    .flex_col()
                    .gap(px(crate::ui::GAP))
                    .children(fields);
                let cancel_button = on_cancel.clone();
                let submit = on_submit.clone();
                let close = on_cancel.clone();
                let disabled = computed(move || {
                    state.with(|state| {
                        state.as_ref().is_none_or(|form| {
                            form.kind == FormKind::StashSelectedFiles
                                && form.selected_stash_paths().is_empty()
                        })
                    })
                });
                let footer = vec![
                    crate::ui::button::button(
                        move || i18n::t(language.get().as_str(), "common.cancel"),
                        crate::ui::button::ButtonVariant::Secondary,
                        crate::ui::button::ButtonSize::Regular,
                        false,
                        move || cancel_button(),
                    )
                    .into_element(),
                    dynamic(move || {
                        let submit = submit.clone();
                        crate::ui::button::button(
                            move || i18n::t(language.get().as_str(), kind.confirm()),
                            crate::ui::button::ButtonVariant::Primary,
                            crate::ui::button::ButtonSize::Regular,
                            disabled.get(),
                            move || {
                                if let Some(form) = state.get_untracked() {
                                    submit(form);
                                }
                            },
                        )
                    })
                    .into_element(),
                ];
                crate::ui::dialog::dialog_panel(
                    420.0,
                    move || i18n::t(language.get().as_str(), kind.title()),
                    Some(std::rc::Rc::new(move || {
                        i18n::t(language.get().as_str(), "common.close")
                    })),
                    body,
                    footer,
                    move || close(),
                )
            })
        },
        move || dismiss(),
    )
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

/// 可选字段变化时复选行会重建，使用按钮复选避免绑定读取已释放的 owner。
fn flag_row(
    language: Signal<SharedString>,
    label_key: &'static str,
    form: FormState,
    state: Signal<Option<FormState>>,
) -> impl IntoElement {
    crate::ui::check::check_row_button(
        move || i18n::t(language.get().as_str(), label_key),
        form.flag,
        move |flag| {
            state.update(|form| {
                if let Some(form) = form {
                    form.flag = flag;
                    form.error = None;
                }
            })
        },
    )
}

fn field(
    language: Signal<SharedString>,
    label_key: &'static str,
    placeholder_key: &'static str,
    state: Signal<Option<FormState>>,
    read: fn(&FormState) -> String,
    write: fn(&mut FormState, String),
) -> impl IntoElement {
    view()
        .w_full()
        .flex_col()
        .gap(px(crate::ui::GAP_XS))
        .child(
            text(move || i18n::t(language.get().as_str(), label_key))
                .text_size(px(crate::ui::SECONDARY))
                .font_weight(FontWeight::SEMIBOLD)
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(design::text2),
        )
        .child(crate::ui::text_input::text_input_bound(
            move || state.with(|form| form.as_ref().map(read).unwrap_or_default()),
            move || i18n::t(language.get().as_str(), placeholder_key),
            false,
            move |value| {
                state.update(|form| {
                    if let Some(form) = form {
                        write(form, value);
                        form.error = None;
                    }
                })
            },
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Props)]
    struct FixtureProps {
        kind: u32,
    }

    #[component]
    fn FormFixture(props: FixtureProps) -> impl IntoElement {
        let state = signal(Some(FormState::new(
            [
                FormKind::NewBranch,
                FormKind::NewTag,
                FormKind::AddRemote,
                FormKind::NewStash,
                FormKind::StashSelectedFiles,
                FormKind::RenameBranch,
                FormKind::EditRemote,
                FormKind::Identity,
                FormKind::NewWorktree,
            ][props.kind as usize],
        )));
        form_dialog(
            signal(SharedString::from("en")),
            state,
            std::sync::Arc::new(move |_| state.set(None)),
            std::sync::Arc::new(move || state.set(None)),
        )
    }

    fn start(kind: FormKind) -> nsh_testing::Headless {
        let mut app = nsh_testing::Headless::start(nsh_engine::EngineConfig {
            viewport: size(px(800.0), px(700.0)),
            ..nsh_engine::EngineConfig::new(nsh_core::element::Root::new::<FormFixture>(
                move || FixtureProps { kind: kind as u32 },
            ))
        })
        .unwrap();
        app.first_render().unwrap();
        app
    }

    #[test]
    fn form_dialogs_share_compact_header_footer_and_placeholders() {
        for kind in [
            FormKind::NewBranch,
            FormKind::NewTag,
            FormKind::AddRemote,
            FormKind::NewStash,
            FormKind::StashSelectedFiles,
            FormKind::RenameBranch,
            FormKind::EditRemote,
            FormKind::Identity,
            FormKind::NewWorktree,
        ] {
            let mut app = start(kind);
            let placeholder = i18n::t("en", kind.placeholder(kind.first_label()));
            assert_ne!(placeholder, kind.placeholder(kind.first_label()));
            assert!(
                app.screen()
                    .input_text(&format!("text:{placeholder}"))
                    .unwrap()
                    .is_some(),
                "{kind:?}: missing placeholder"
            );
            let nodes = app.screen().layout_nodes();
            let height = |id: &str| {
                nodes
                    .iter()
                    .find(|node| node.test_id.as_deref() == Some(id))
                    .unwrap()
                    .bounds
                    .size
                    .height
                    .0
            };
            assert!((height("dialog-header") - 44.0).abs() < 1.0);
            assert!((height("dialog-footer") - 48.0).abs() < 1.0);
            if kind == FormKind::NewStash {
                assert!(height("dialog-panel") < 260.0);
            }
            if kind == FormKind::NewBranch {
                assert!(height("dialog-panel") < 330.0);
            }
            app.press("Cancel").unwrap();
            assert!(
                !app.screen()
                    .layout_nodes()
                    .iter()
                    .any(|node| node.test_id.as_deref() == Some("dialog-panel"))
            );
        }
    }

    #[test]
    fn form_checkbox_keeps_typed_values_and_closes_cleanly() {
        let mut app = start(FormKind::NewStash);
        let placeholder = i18n::t("en", "form.placeholders.stashMessage");
        app.type_text(&placeholder, "work in progress").unwrap();
        app.press(&i18n::t("en", "form.includeUntracked")).unwrap();
        assert_eq!(
            app.screen().input_text("work in progress").unwrap(),
            Some("work in progress")
        );
        app.press("Cancel").unwrap();
        assert!(
            !app.screen()
                .layout_nodes()
                .iter()
                .any(|node| node.test_id.as_deref() == Some("dialog-panel"))
        );
    }

    #[test]
    fn typing_in_branch_form_preserves_the_input_view() {
        let mut app = start(FormKind::NewBranch);
        let input_ids = |app: &nsh_testing::Headless| {
            app.screen()
                .layout_nodes()
                .into_iter()
                .filter(|node| node.kind == nsh_core::ViewKind::Input)
                .map(|node| node.id)
                .collect::<Vec<_>>()
        };
        let before = input_ids(&app);
        app.type_text("feature/my-change", "topic").unwrap();
        assert_eq!(app.screen().input_text("topic").unwrap(), Some("topic"));
        assert_eq!(input_ids(&app), before, "编辑字段不重建输入框");
    }
}
