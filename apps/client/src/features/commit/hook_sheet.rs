//! 钩子完整日志弹层（设计板 10 的 `View full log`）。
//!
//! 面板里只放末尾几行做提示，长输出（尤其是失败时的编译日志）放在这里看，
//! 并提供复制与关闭。

use super::commands::CommitCommands;
use super::state::CommitState;
use crate::app::session::Tab;
use crate::design;
use crate::i18n;
use nsh_core::prelude::*;

/// 日志弹层：居中、可滚动、Esc 或点背景关闭。
///
/// 入口 `hook_sheet` 收本功能的状态与用例；弹层读取的钩子输出挂在 `Tab` 上，
/// 复制到剪贴板走 `shared::os` 的共用函数。
pub(crate) fn hook_sheet(
    _state: &CommitState,
    cmd: &CommitCommands,
    language: Signal<SharedString>,
    tab: Memo<Option<Tab>>,
) -> impl IntoElement {
    let close = {
        let cmd = cmd.clone();
        move |open: bool| cmd.toggle_hook_log(open)
    };
    let notice = cmd.notice;
    let dismiss = close.clone();
    return crate::ui::dialog::modal(
        move || tab.get().map(|tab| tab.hook_open).unwrap_or(false),
        move || i18n::t(language.get().as_str(), "commit.logTitle"),
        120,
        move || {
            let tab = tab;
            let language = language;
            let close = close.clone();
            let notice = notice;
            dynamic(move || -> AnyElement {
                let Some(tab) = tab.get() else {
                    return view().w(px(0.0)).h(px(0.0)).into_element();
                };
                if !tab.hook_open {
                    return view().w(px(0.0)).h(px(0.0)).into_element();
                }
                let log = tab.hook_log.clone().unwrap_or_default();
                let language_value = language;
                let close = close.clone();
                let close_button = close.clone();
                let log_for_copy = log.clone();
                let body = scroll().w_full().h(px(300.0)).child(
                    text(log)
                        .select_text()
                        .text_size(px(crate::ui::CODE))
                        .line_height(px(crate::ui::CODE_LINE))
                        .text_color(design::text2),
                );
                let footer = vec![
                    crate::ui::button::button(
                        move || i18n::t(language_value.get().as_str(), "common.copy"),
                        crate::ui::button::ButtonVariant::Secondary,
                        crate::ui::button::ButtonSize::Regular,
                        false,
                        move || {
                            crate::shared::os::copy_text(
                                notice,
                                language_value,
                                log_for_copy.clone(),
                            )
                        },
                    )
                    .into_element(),
                    crate::ui::button::button(
                        move || i18n::t(language_value.get().as_str(), "common.close"),
                        crate::ui::button::ButtonVariant::Secondary,
                        crate::ui::button::ButtonSize::Regular,
                        false,
                        move || close_button(false),
                    )
                    .into_element(),
                ];
                return crate::ui::dialog::dialog_panel(
                    560.0,
                    move || i18n::t(language_value.get().as_str(), "commit.logTitle"),
                    Some(std::rc::Rc::new(move || {
                        i18n::t(language_value.get().as_str(), "common.close")
                    })),
                    body,
                    footer,
                    move || close(false),
                );
            })
        },
        move || dismiss(false),
    );
}
