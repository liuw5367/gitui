//! 下拉选择：行为来自 NSH Select，触发器与选项样式由应用统一提供。

use crate::design;
use crate::ui;
use nsh_base::{SelectEntry, SelectItem};
use nsh_core::prelude::*;
use std::rc::Rc;

/// 一个选项：稳定 id 与随语言变化的文案。
pub(crate) struct SelectOption {
    pub(crate) id: &'static str,
    pub(crate) label: Box<dyn Fn() -> String>,
}

impl SelectOption {
    pub(crate) fn new(id: &'static str, label: impl Fn() -> String + 'static) -> SelectOption {
        return SelectOption {
            id,
            label: Box::new(label),
        };
    }
}

/// 下拉字段。`width` 为 `None` 时占满父容器。
pub(crate) fn select(
    options: Vec<SelectOption>,
    selected: impl Into<SharedString>,
    width: Option<f32>,
    disabled: bool,
    on_select: impl Fn(&'static str) + 'static,
) -> impl IntoElement {
    let options = Rc::new(options);
    let on_select = Rc::new(on_select);
    let selected = selected.into();
    let selected_id = selected.clone();
    let current = options
        .iter()
        .find(|option| option.id == selected.as_ref())
        .map(|option| (option.label)())
        .unwrap_or_default();
    let accessibility = SharedString::from(current);
    let items = options
        .iter()
        .map(|option| SelectItem::new(option.id, (option.label)()))
        .collect::<Vec<_>>();
    let trigger_options = options.clone();
    let trigger_selected = selected.clone();
    let change_options = options.clone();
    let choose = on_select.clone();

    let mut trigger = view()
        .h(px(ui::CONTROL))
        .px(px(8.0))
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(move || design::field_border())
        .bg(move || design::field())
        .hover(move |style| style.bg(move || design::hover()))
        .focus_visible(move |style| {
            style
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        })
        .style_when(move || disabled, |style| style.opacity(0.45))
        .child(
            text(move || {
                trigger_options
                    .iter()
                    .find(|option| option.id == trigger_selected.as_ref())
                    .map(|option| (option.label)())
                    .unwrap_or_default()
            })
            .flex_1()
            .min_w_0()
            .text_size(px(ui::SECONDARY))
            .line_height(px(ui::CONTROL))
            .text_color(move || design::text())
            .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(ui::icon::icon_view(
            "chev-down",
            ui::icon::IconSize::Compact,
            design::text3,
        ));
    if let Some(width) = width {
        trigger = trigger.w(px(width)).flex_shrink_0();
    } else {
        trigger = trigger.w_full();
    }

    return nsh_base::select()
        .items(items)
        .value(move || Some(selected_id.clone()))
        .on_change(move |value: &Option<SharedString>| {
            let Some(value) = value else {
                return;
            };
            if let Some(option) = change_options
                .iter()
                .find(|option| option.id == value.as_ref())
            {
                choose(option.id);
            }
        })
        .disabled(disabled)
        .accessibility_label(accessibility)
        .offset(px(4.0))
        .popup_style(|style| {
            style
                .p(px(4.0))
                .rounded(px(8.0))
                .border_1()
                .border_color(move || design::line())
                .bg(move || design::token("popover"))
                .text_color(move || design::text())
                .shadow_md()
        })
        .item_class("rounded-md focus:bg-accent selected:font-medium disabled:opacity-50")
        .render_item(|item: &SelectEntry| {
            view().h(px(ui::CONTROL)).px(px(8.0)).items_center().child(
                text(item.title.clone())
                    .text_size(px(ui::SECONDARY))
                    .line_height(px(ui::CONTROL))
                    .text_color(move || design::text()),
            )
        })
        .trigger(trigger);
}
