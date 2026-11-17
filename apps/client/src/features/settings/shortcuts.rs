//! 设置页的快捷键分区（设计板 25）：搜索、录制、按 Global / Repository 分组。
//!
//! 分组与行数来自 `shortcuts::ACTIONS` 的真实注册表：本构建有 22 个可绑定动作，
//! 画板的 4 / 6 / 3 只是示例，不能为了对齐数量伪造动作。

use crate::design;
use crate::shortcuts::{self, Scope};
use crate::ui;
use crate::i18n;
use nsh_core::prelude::*;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub(super) fn shortcuts_page(
    language: Signal<SharedString>,
    registry: Signal<shortcuts::Registry>,
    query: Signal<String>,
    recording: Signal<Option<&'static str>>,
    error: Signal<Option<String>>,
    record: Arc<dyn Fn(&'static str)>,
    restore: Arc<dyn Fn()>,
) -> AnyElement {
    let search = ui::text_input::search_field(
        query,
        {
            let language = language;
            move || i18n::t(language.get().as_str(), "shortcut.search")
        },
        None,
        move |text: &str| query.set(text.to_string()),
    );
    let restore_button = ui::button::button(
        {
            let language = language;
            move || i18n::t(language.get().as_str(), "shortcut.restoreDefaults")
        },
        ui::button::ButtonVariant::Secondary,
        ui::button::ButtonSize::Small,
        false,
        {
            let restore = restore.clone();
            move || restore()
        },
    );
    let body = dynamic(move || {
        body(
            language,
            registry,
            query,
            recording,
            error,
            record.clone(),
        )
        .into_element()
    });
    return view()
        .w_full()
        .flex_col()
        .child(
            view()
                .w_full()
                .mt(px(14.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(view().w(px(240.0)).flex_shrink_0().child(search))
                .child(view().flex_1())
                .child(restore_button),
        )
        .child(
            text({
                let language = language;
                move || i18n::t(language.get().as_str(), "shortcut.recordingHint")
            })
            .mt(px(6.0))
            .text_size(px(ui::SECONDARY))
            .line_height(px(ui::SECONDARY_LINE))
            .text_color(move || design::text3()),
        )
        .child(body)
        .into_element();
}

#[allow(clippy::too_many_arguments)]
fn body(
    language: Signal<SharedString>,
    registry: Signal<shortcuts::Registry>,
    query: Signal<String>,
    recording: Signal<Option<&'static str>>,
    error: Signal<Option<String>>,
    record: Arc<dyn Fn(&'static str)>,
) -> AnyElement {
    let _ = language.get();
    let needle = query.get().to_lowercase();
    let bindings = registry.get();
    let recording_id = recording.get();
    let error_text = error.get();
    let mut body = view().w_full().flex_col();
    if let Some(id) = recording_id {
        body = body.child(recorder(language, id, bindings.clone()));
    }
    if let Some(text_value) = error_text {
        body = body.child(
            text(move || text_value.clone())
                .mt(px(6.0))
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::danger()),
        );
    }
    let mut any = false;
    for scope in [Scope::Global, Scope::Repository] {
        let mut rows: Vec<AnyElement> = Vec::new();
        for action in shortcuts::ACTIONS
            .iter()
            .filter(|action| action.scope == scope)
        {
            let label = shortcuts::label(action, language.get_untracked().as_str());
            let combo = bindings.combo(action.id).to_string();
            let searchable = format!("{label} {combo}").to_lowercase();
            if !needle.is_empty() && !searchable.contains(&needle) {
                continue;
            }
            any = true;
            let id = action.id;
            let record = record.clone();
            rows.push(
                ui::setting_row::setting_row(
                    move || shortcuts::label(action, language.get().as_str()),
                    Some(Box::new(move || {
                        i18n::t(language.get().as_str(), action.desc)
                    })),
                    keycaps_control(language, combo.as_str(), id, record),
                    true,
                )
                .into_element(),
            );
        }
        if rows.is_empty() {
            continue;
        }
        let count = rows.len();
        let mut section = view()
            .w_full()
            .flex_col()
            .mt(px(18.0))
            .child(
                view()
                    .w_full()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        text(move || {
                            i18n::t(language.get().as_str(), scope_key(scope))
                        })
                        .text_size(px(ui::BODY))
                        .line_height(px(ui::BODY_LINE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || design::text()),
                    )
                    .child(
                        text(move || {
                            i18n::with_arguments(
                                language.get().as_str(),
                                "shortcut.count",
                                &[("count", count.to_string())],
                            )
                        })
                        .text_size(px(ui::SECONDARY))
                        .line_height(px(ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                    ),
            )
            .child(view().w_full().flex_col().child(rows));
        body = body.child(section);
    }
    if !any {
        body = body.child(
            text(move || i18n::t(language.get().as_str(), "shortcut.searchEmpty"))
                .mt(px(18.0))
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        );
    }
    return body.into_element();
}

fn scope_key(scope: Scope) -> &'static str {
    return match scope {
        Scope::Global => "shortcut.groupGlobal",
        Scope::Repository => "shortcut.groupRepository",
    };
}

fn keycaps_control(
    language: Signal<SharedString>,
    combo: &str,
    id: &'static str,
    record: Arc<dyn Fn(&'static str)>,
) -> AnyElement {
    let caps = shortcuts::keycaps(combo);
    let mut keys = view().flex_row().items_center().gap(px(2.0));
    if caps.is_empty() {
        keys = keys.child(
            text(move || i18n::t(language.get().as_str(), "shortcut.unassigned"))
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        );
    } else {
        for cap in caps {
            keys = keys.child(ui::badge::kbd(cap));
        }
    }
    return view()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .child(keys)
        .child(ui::button::button(
            move || i18n::t(language.get().as_str(), "shortcut.record"),
            ui::button::ButtonVariant::Ghost,
            ui::button::ButtonSize::Small,
            false,
            move || record(id),
        ))
        .into_element();
}

fn recorder(
    language: Signal<SharedString>,
    id: &'static str,
    bindings: shortcuts::Registry,
) -> AnyElement {
    let action = shortcuts::ACTIONS
        .iter()
        .find(|action| action.id == id)
        .expect("recording action");
    let name = shortcuts::label(action, language.get_untracked().as_str());
    let combo = bindings.combo(id).to_string();
    let caps = shortcuts::keycaps(combo.as_str());
    let mut keys = view().flex_row().items_center().gap(px(2.0));
    if caps.is_empty() {
        keys = keys.child(
            text(move || i18n::t(language.get().as_str(), "shortcut.unassigned"))
                .text_size(px(ui::SECONDARY))
                .text_color(move || design::text3()),
        );
    } else {
        for cap in caps {
            keys = keys.child(ui::badge::kbd(cap));
        }
    }
    keys = keys.child(
        view()
            .w(px(2.0))
            .h(px(16.0))
            .flex_shrink_0()
            .bg(move || design::accent()),
    );
    return view()
        .w_full()
        .mt(px(10.0))
        .p(px(10.0))
        .flex_col()
        .gap(px(6.0))
        .rounded(px(8.0))
        .border_1()
        .border_color(move || design::accent())
        .bg(move || design::accent_soft())
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    view()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded(px(3.0))
                        .bg(move || design::danger()),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "shortcut.recordingTitle"))
                        .text_size(px(ui::SECONDARY))
                        .line_height(px(ui::SECONDARY_LINE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || design::accent()),
                )
                .child(
                    text(move || format!("· {name}"))
                        .text_size(px(ui::SECONDARY))
                        .line_height(px(ui::SECONDARY_LINE))
                        .text_color(move || design::text()),
                ),
        )
        .child(refine_row(keys))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .child(
                    text(move || i18n::t(language.get().as_str(), "shortcut.pressNew"))
                        .text_size(px(ui::SECONDARY))
                        .line_height(px(ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .child(ui::badge::kbd("esc"))
                .child(
                    text(move || i18n::t(language.get().as_str(), "shortcut.cancel"))
                        .text_size(px(ui::SECONDARY))
                        .line_height(px(ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .child(ui::badge::kbd("⌫"))
                .child(
                    text(move || i18n::t(language.get().as_str(), "shortcut.clear"))
                        .text_size(px(ui::SECONDARY))
                        .line_height(px(ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                ),
        )
        .into_element();
}

fn refine_row(keys: impl IntoElement) -> impl IntoElement {
    return view()
        .w_full()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .child(keys);
}
