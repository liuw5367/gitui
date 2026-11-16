//! 设置页的快捷键分区（设计板 25）：列出本构建真实的按键绑定，支持搜索、录制与
//! 恢复默认。

use crate::design::{self, metrics};
use crate::i18n;
use nsh_core::prelude::*;
use std::sync::Arc;

/// 快捷键分区（25）：列出本构建真实的按键绑定。
#[allow(clippy::too_many_arguments)]
pub(super) fn shortcuts_page(
    language: Signal<SharedString>,
    registry: Signal<crate::shortcuts::Registry>,
    query: Signal<String>,
    recording: Signal<Option<&'static str>>,
    error: Signal<Option<String>>,
    record: Arc<dyn Fn(&'static str)>,
    restore: Arc<dyn Fn()>,
) -> AnyElement {
    let needle = query.get_untracked().to_lowercase();
    let bindings = registry.get_untracked();
    let mut rows: Vec<AnyElement> = Vec::new();
    for action in crate::shortcuts::ACTIONS.iter() {
        let label = crate::shortcuts::label(action, language.get_untracked().as_str());
        if !needle.is_empty() && !label.to_lowercase().contains(&needle) {
            continue;
        }
        let id = action.id;
        let is_default = bindings.combo(id) == action.default;
        let record = record.clone();
        rows.push(
            view()
                .w_full()
                .h(px(26.0))
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .child(
                    text(label)
                        .flex_1()
                        .text_size(px(crate::ui::BODY))
                        .line_height(px(15.0))
                        .text_color(move || design::text()),
                )
                .child(
                    text(move || {
                        if is_default {
                            String::new()
                        } else {
                            i18n::t(language.get().as_str(), "settings.customized")
                        }
                    })
                    .text_size(px(crate::ui::META))
                    .line_height(px(crate::ui::SECONDARY_LINE))
                    .text_color(move || design::text3()),
                )
                .child(
                    text(move || {
                        let value = registry.get().combo(id).to_string();
                        crate::shortcuts::pretty(value.as_str())
                    })
                    .text_size(px(crate::ui::CODE))
                    .line_height(px(14.0))
                    .text_color(move || design::text2()),
                )
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "shortcut.record"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let record = record.clone();
                        move || record(id)
                    },
                ))
                .into_element(),
        );
    }
    if rows.is_empty() {
        rows.push(
            text(move || i18n::t(language.get().as_str(), "shortcut.searchEmpty"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3())
                .into_element(),
        );
    }
    return view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        .child(
            text(move || i18n::t(language.get().as_str(), "settings.shortcutsHint"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
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
                .child(crate::ui::icon::icon_view(
                    "search",
                    crate::ui::icon::IconSize::Button,
                    design::text3,
                ))
                .child(
                    nsh_base::input()
                        .value(query)
                        .on_change(move |text: &str| query.set(text.to_string()))
                        .placeholder(move || i18n::t(language.get().as_str(), "shortcut.search"))
                        .text_size(px(crate::ui::BODY))
                        .flex_1(),
                ),
        )
        // 录制提示与改键错误：两者都只在有内容时占高度。
        .child(
            view()
                .w_full()
                .flex_col()
                .gap(px(2.0))
                .child(
                    text(move || match recording.get() {
                        Some(id) => {
                            let name = crate::shortcuts::ACTIONS
                                .iter()
                                .find(|action| action.id == id)
                                .map(|action| {
                                    crate::shortcuts::label(
                                        action,
                                        language.get_untracked().as_str(),
                                    )
                                })
                                .unwrap_or_default();
                            i18n::with_arguments(
                                language.get().as_str(),
                                "shortcut.recording",
                                &[("action", name)],
                            )
                        }
                        None => String::new(),
                    })
                    .text_size(px(crate::ui::SECONDARY))
                    .line_height(px(crate::ui::SECONDARY_LINE))
                    .text_color(move || design::accent()),
                )
                .child(
                    text(move || error.get().unwrap_or_default())
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::danger()),
                ),
        )
        .child(view().w_full().flex_col().gap(px(2.0)).child(rows))
        .child(
            view()
                .w_full()
                .flex_row()
                .gap(px(8.0))
                .child(view().flex_1())
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "shortcut.restoreDefaults"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Regular,
                    false,
                    {
                        let restore = restore.clone();
                        move || restore()
                    },
                )),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), "shortcut.catalogHint"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .into_element();
}
