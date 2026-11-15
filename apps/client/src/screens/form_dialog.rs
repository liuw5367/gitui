//! 表单对话框（设计板 22）：新建分支 / 标签 / 远程 / 贮藏。
//!
//! `alert_dialog` 只有按钮没有输入框，这里自建一个居中的模态浮层，带 1–2 个输入
//! 字段与取消/确认。确认后由调用方执行真正的 Git 操作。

use crate::design::{self, metrics};
use crate::i18n;
use nsh_core::prelude::*;

/// 对话框的种类，决定标题、字段与确认文案。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormKind {
    NewBranch,
    NewTag,
    AddRemote,
    NewStash,
    RenameBranch,
    EditRemote,
    Identity,
}

impl FormKind {
    fn title(self) -> &'static str {
        return match self {
            FormKind::NewBranch => "form.newBranch",
            FormKind::NewTag => "form.newTag",
            FormKind::AddRemote => "form.addRemote",
            FormKind::NewStash => "form.newStash",
            FormKind::RenameBranch => "form.renameBranch",
            FormKind::EditRemote => "form.editRemote",
            FormKind::Identity => "form.identity",
        };
    }

    /// 第一个字段的标签。
    fn first_label(self) -> &'static str {
        return match self {
            FormKind::NewBranch | FormKind::NewTag => "form.name",
            FormKind::AddRemote => "form.remoteName",
            FormKind::NewStash => "form.message",
            FormKind::RenameBranch => "form.newName",
            FormKind::EditRemote => "form.urlNew",
            FormKind::Identity => "form.authorName",
        };
    }

    /// 第三个字段的标签（只有附注标签需要说明）。
    fn third_label(self) -> Option<&'static str> {
        return match self {
            FormKind::NewTag => Some("form.message"),
            _ => None,
        };
    }

    /// 复选项的标签；`None` 表示没有复选。
    fn flag_label(self) -> Option<&'static str> {
        return match self {
            FormKind::NewBranch => Some("form.checkoutAfter"),
            FormKind::NewTag => Some("form.annotated"),
            FormKind::NewStash => Some("form.includeUntracked"),
            _ => None,
        };
    }

    /// 复选的初始值：贮藏默认带上未跟踪文件，其它默认不勾。
    fn flag_default(self) -> bool {
        return matches!(self, FormKind::NewStash);
    }

    /// 第二个字段的标签；`None` 表示只有一个字段。
    fn second_label(self) -> Option<&'static str> {
        return match self {
            FormKind::NewBranch | FormKind::NewTag => Some("form.from"),
            FormKind::AddRemote => Some("form.url"),
            FormKind::NewStash => None,
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
            FormKind::RenameBranch => "form.rename",
            FormKind::EditRemote => "form.save",
            FormKind::Identity => "form.save",
        };
    }
}

/// 对话框状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FormState {
    pub kind: FormKind,
    pub first: String,
    pub second: String,
    /// 第三个字段（目前只有附注标签的说明用）。
    pub third: String,
    /// 复选项的值；含义由 [`FormKind::flag_label`] 决定。
    pub flag: bool,
    pub error: Option<String>,
}

impl FormState {
    pub fn new(kind: FormKind) -> FormState {
        return FormState {
            kind,
            first: String::new(),
            second: String::new(),
            third: String::new(),
            flag: kind.flag_default(),
            error: None,
        };
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
    return crate::ui::dialog::scrim(
        move || state.get().is_some(),
        120,
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
                    let Some(label_key) = form.kind.third_label() else {
                        return view().w_full().h(px(0.0)).into_element();
                    };
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
                    let form_for_flag = form_for_flag.clone();
                    return flag_row(language, label_key, form_for_flag, state).into_element();
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
                                state
                                    .get()
                                    .and_then(|form| form.error)
                                    .unwrap_or_default()
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
                                          false,
                                          {
                                let on_submit = on_submit.clone();
                                move || on_submit(submit_form.clone())
                            },
                                      )),
                )
                .into_element(),
            )
            .into_element();
        }),
        move || dismiss(),
    );
}

/// 一行复选：走 `widgets::check`（原生 Checkbox + 统一指示器）。
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
    return view()
        .w_full()
        .h(px(22.0))
        .flex_row()
        .items_center()
        .child(crate::ui::check::check_inline(
            move || i18n::t(language.get().as_str(), label_key),
            form.flag,
            22.0,
            14.0,
            false,
            toggle,
        ));
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
        .child(
            view()
                .w_full()
                .h(px(metrics::FIELD_H))
                .flex_row()
                .items_center()
                .px(px(8.0))
                .rounded(px(metrics::RADIUS_CONTROL))
                .border_1()
                .border_color(move || design::field_border())
                .bg(move || design::field())
                .child(crate::ui::text_input::text_input_bound(
                    {
                        let form = form.clone();
                        move || read(&form)
                    },
                    || String::new(),
                    false,
                    move |text: String| write(text),
                )),
        );
}
