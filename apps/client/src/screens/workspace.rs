//! 工作区：标题栏 + 工具栏 + 侧栏 + History/diff + 状态栏。
//!
//! P1 只读；打开失败时在整个正文区显示错误面板，而不是留在空白界面。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
use crate::shell;
use crate::workspace::{diff, history, sidebar};
use crate::Actions;
use nsh_core::prelude::*;

pub(crate) fn workspace(
    language: Signal<SharedString>,
    tabs: Signal<Vec<Tab>>,
    active: Signal<usize>,
    history_query: Signal<String>,
    actions: Actions,
) -> impl IntoElement {
    let current = computed(move || tabs.get().get(active.get()).cloned());
    let change_count = move || {
        return current
            .get()
            .map(|tab| tab.status.len() as u32)
            .unwrap_or(0);
    };
    let actions_for_body = actions.clone();
    return view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(move || design::content())
        .child(shell::titlebar(
            language,
            tabs,
            active,
            actions.clone(),
        ))
        .child(shell::toolbar(
            language,
            history_query,
            change_count,
        ))
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex_row()
                .child(match_body(
                    language,
                    current,
                    history_query,
                    actions_for_body,
                )),
        )
        .child(shell::statusbar(language, current));
}

fn match_body(
    language: Signal<SharedString>,
    current: Memo<Option<Tab>>,
    history_query: Signal<String>,
    actions: Actions,
) -> impl IntoElement {
    return view()
        .w_full()
        .h_full()
        .flex_row()
        .child(dynamic(move || -> AnyElement {
            let tab = current.get();
            let has_error = tab
                .as_ref()
                .map(|tab| tab.error.is_some())
                .unwrap_or(false);
            if has_error {
                let detail = tab
                    .and_then(|tab| tab.error)
                    .unwrap_or_else(|| i18n::t(language.get().as_str(), "error.title"));
                let actions = actions.clone();
                return error_panel(language, detail, actions).into_element();
            }
            let Some(tab) = tab else {
                return view()
                    .w_full()
                    .h_full()
                    .items_center()
                    .justify_center()
                    .child(
                        text(move || i18n::t(language.get().as_str(), "history.empty"))
                            .text_size(px(12.0))
                            .text_color(move || design::text3()),
                    )
                    .into_element();
            };
            let actions = actions.clone();
            return body(language, current, tab, history_query, actions).into_element();
        }));
}

fn body(
    language: Signal<SharedString>,
    current: Memo<Option<Tab>>,
    tab: Tab,
    history_query: Signal<String>,
    actions: Actions,
) -> impl IntoElement {
    let tab_for_sidebar = tab.clone();
    let tab_for_history = tab.clone();
    let tab_for_diff = tab;
    return view()
        .w_full()
        .h_full()
        .flex_row()
        .child(sidebar::sidebar(language, tab_for_sidebar))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex_col()
                .child(history::history(
                    language,
                    tab_for_history,
                    history_query,
                    actions.clone(),
                ))
                .child(diff::diff(language, tab_for_diff)),
        )
        .child(view().w_full().h(px(0.0)));
}

fn error_panel(
    language: Signal<SharedString>,
    detail: String,
    actions: Actions,
) -> impl IntoElement {
    let detail = std::sync::Arc::new(detail);
    let retry = actions.refresh.clone();
    let welcome = actions.go_welcome.clone();
    return view()
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .flex_col()
        .gap(px(10.0))
        .child(widgets_icon())
        .child(
            text(move || i18n::t(language.get().as_str(), "error.title"))
                .text_size(px(13.0))
                .font_weight(FontWeight(600))
                .text_color(move || design::text()),
        )
        .child(
            text({
                let detail = detail.clone();
                move || detail.as_str().to_string()
            })
            .text_size(px(11.0))
            .text_color(move || design::text2()),
        )
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(crate::widgets::button(
                    move || i18n::t(language.get().as_str(), "error.retry"),
                    {
                        let retry = retry.clone();
                        move || retry()
                    },
                ))
                .child(crate::widgets::button(
                    move || i18n::t(language.get().as_str(), "error.close"),
                    {
                        let welcome = welcome.clone();
                        move || welcome()
                    },
                )),
        );
}

fn widgets_icon() -> impl IntoElement {
    return crate::widgets::icon("info", 20.0, design::danger);
}
