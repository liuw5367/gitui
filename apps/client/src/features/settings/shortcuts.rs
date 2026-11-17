//! 设置页的快捷键分区（设计板 25）：搜索、录制、按 Global / Repository 分组。
//!
//! 分组与行数来自 `shortcuts::ACTIONS` 的真实注册表：本构建有 22 个可绑定动作，
//! 画板的 4 / 6 / 3 只是示例，不能为了对齐数量伪造动作。
//!
//! 录制卡插在**正在录制的那一行下方**（画板把它画在列表顶部），被搜索过滤掉时才
//! 回落到列表顶部。按键绑定居中在 `app::hotkeys::install_settings_window`，随窗口
//! 内容挂载一次：`on_key` 不能从会被重建的动态子树里可靠地重复绑定。

use crate::design;
use crate::i18n;
use crate::shortcuts::{self, Scope};
use crate::ui;
use nsh_core::prelude::*;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub(super) fn shortcuts_page(
    language: Signal<SharedString>,
    registry: Signal<shortcuts::Registry>,
    query: Signal<String>,
    recording: Signal<Option<&'static str>>,
    error: Signal<Option<String>>,
    pending: Signal<Option<String>>,
    blink: Signal<bool>,
    record: Arc<dyn Fn(&'static str)>,
    restore: Arc<dyn Fn()>,
    clear: Arc<dyn Fn(&'static str)>,
    reset: Arc<dyn Fn(&'static str)>,
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
            pending,
            blink,
            record.clone(),
            clear.clone(),
            reset.clone(),
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
    pending: Signal<Option<String>>,
    blink: Signal<bool>,
    record: Arc<dyn Fn(&'static str)>,
    clear: Arc<dyn Fn(&'static str)>,
    reset: Arc<dyn Fn(&'static str)>,
) -> AnyElement {
    let _ = language.get();
    let needle = query.get().to_lowercase();
    let bindings = registry.get();
    let recording_id = recording.get();
    let error_text = error.get();
    let pending = pending.get();
    // 录制卡默认插在正在录制的那一行下方；只有该行被搜索过滤掉时才退回到列表顶部，
    // 否则录制中会看不到取消入口。
    let mut recorder_card = recording_id
        .map(|id| {
            recorder(
                language,
                id,
                bindings.clone(),
                pending.clone(),
                blink,
                error_text.clone(),
            )
        });
    let mut sections: Vec<AnyElement> = Vec::new();
    for scope in [Scope::Global, Scope::Repository] {
        let visible = shortcuts::ACTIONS
            .iter()
            .filter(|action| action.scope == scope)
            .filter(|action| {
                let label = shortcuts::label(action, language.get_untracked().as_str());
                let combo = bindings.combo(action.id);
                let searchable = format!("{label} {combo}").to_lowercase();
                needle.is_empty() || searchable.contains(&needle)
            })
            .collect::<Vec<_>>();
        let count = visible.len();
        if count == 0 {
            continue;
        }
        let first = sections.is_empty();
        let mut rows: Vec<AnyElement> = Vec::new();
        for (index, action) in visible.into_iter().enumerate() {
            let last = index + 1 == count;
            let id = action.id;
            let is_recording = recording_id == Some(id);
            let record = record.clone();
            let clear = clear.clone();
            let reset = reset.clone();
            rows.push(
                ui::setting_row::setting_row(
                    move || shortcuts::label(action, language.get().as_str()),
                    Some(Box::new(move || {
                        i18n::t(language.get().as_str(), action.desc)
                    })),
                    keycaps_control(language, bindings.combo(id), id, record, clear, reset),
                    !last && !is_recording,
                )
                .into_element(),
            );
            if is_recording && let Some(card) = recorder_card.take() {
                rows.push(card);
            }
        }
        let count_text = text(move || {
            i18n::with_arguments(
                language.get().as_str(),
                "shortcut.count",
                &[("count", count.to_string())],
            )
        })
        .text_size(px(ui::SECONDARY))
        .line_height(px(ui::SECONDARY_LINE))
        .text_color(move || design::text3());
        sections.push(
            ui::setting_row::settings_section_titled(
                first,
                move || i18n::t(language.get().as_str(), scope_key(scope)),
                Some(count_text.into_element()),
                None,
                view().w_full().flex_col().child(rows),
            )
            .into_element(),
        );
    }
    let mut body = view().w_full().flex_col();
    if let Some(card) = recorder_card {
        body = body.child(card);
    }
    let empty = sections.is_empty();
    for section in sections {
        body = body.child(section);
    }
    if empty && recording_id.is_none() {
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
    clear: Arc<dyn Fn(&'static str)>,
    reset: Arc<dyn Fn(&'static str)>,
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
        .child(ui::button::button(
            move || i18n::t(language.get().as_str(), "shortcut.reset"),
            ui::button::ButtonVariant::Ghost,
            ui::button::ButtonSize::Small,
            false,
            move || reset(id),
        ))
        .child(ui::button::button(
            move || i18n::t(language.get().as_str(), "shortcut.clear"),
            ui::button::ButtonVariant::Ghost,
            ui::button::ButtonSize::Small,
            false,
            move || clear(id),
        ))
        .into_element();
}

fn recorder(
    language: Signal<SharedString>,
    id: &'static str,
    bindings: shortcuts::Registry,
    pending: Option<String>,
    blink: Signal<bool>,
    error: Option<String>,
) -> AnyElement {
    let action = shortcuts::ACTIONS
        .iter()
        .find(|action| action.id == id)
        .expect("recording action");
    let name = shortcuts::label(action, language.get_untracked().as_str());
    // 优先显示刚按下的组合：录制中它是用户最需要确认的东西。
    let combo = pending.unwrap_or_else(|| bindings.combo(id).to_string());
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
    // 录制中闪一下，明示「正在等按键」；不录制时它不出现（卡片本身就是录制态）。
    keys = keys.child(
        view()
            .w(px(2.0))
            .h(px(16.0))
            .flex_shrink_0()
            .bg(move || {
                if blink.get() {
                    design::accent()
                } else {
                    AdaptiveColor::TRANSPARENT
                }
            }),
    );
    let mut card = view()
        .w_full()
        .mt(px(4.0))
        .mb(px(6.0))
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
        );
    // 冲突与不可绑定只发生在录制中：错误贴着录制卡显示，录制状态保留，可直接改按别的组合。
    if let Some(message) = error {
        card = card.child(
            text(move || message.clone())
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::danger()),
        );
    }
    return card.into_element();
}

fn refine_row(keys: impl IntoElement) -> impl IntoElement {
    return view()
        .w_full()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .child(keys);
}
