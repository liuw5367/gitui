//! 下拉选择：28 高描边字段 + 当前值 + chevron，点按弹出系统菜单。
//!
//! 设计板的 `.set-select` 是原生 select；NSH 0.5.0 没有可从 Rust 直接套皮的
//! select trigger，因此这里用 `nsh_base::button` 提供焦点 / 无障碍 / 禁用态，
//! 用系统 `NSMenu` 提供真正的原生弹出列表。外观与 26 高的字段一致，高度按
//! `DEVIATIONS #4` 取系统控件刻度 28。

use crate::design;
use crate::ui;
use nsh_core::element::desktop::MenuItem;
use nsh_core::prelude::*;
use std::rc::Rc;

/// 一个选项：稳定 id 与随语言变化的文案。
pub(crate) struct SelectOption {
    pub(crate) id: &'static str,
    pub(crate) label: Box<dyn Fn() -> String>,
}

impl SelectOption {
    pub(crate) fn new(
        id: &'static str,
        label: impl Fn() -> String + 'static,
    ) -> SelectOption {
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
    let accessibility = current.clone();
    let for_menu = options.clone();
    let choose = on_select.clone();
    let mut field = nsh_base::button()
        .accessibility_label(Some(SharedString::from(accessibility)))
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
        .on_press(move |_: &()| {
            let items: Vec<MenuItem> = for_menu
                .iter()
                .map(|option| {
                    let label = (option.label)();
                    let id = option.id;
                    let choose = choose.clone();
                    return MenuItem::new(label)
                        .checked(id == selected_id.as_ref())
                        .on_select(move || choose(id));
                })
                .collect();
            let _ = nsh_core::element::desktop::popup_menu(items);
        });
    if let Some(width) = width {
        field = field.w(px(width)).flex_shrink_0();
    } else {
        field = field.w_full();
    }
    return field
        .child(
            text(move || {
                options
                    .iter()
                    .find(|option| option.id == selected.as_ref())
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
}
