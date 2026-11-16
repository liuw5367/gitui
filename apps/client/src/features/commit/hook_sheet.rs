//! 钩子完整日志弹层（设计板 10 的 `View full log`）。
//!
//! 面板里只放末尾几行做提示，长输出（尤其是失败时的编译日志）放在这里看，
//! 并提供复制与关闭。

use super::commands::CommitCommands;
use super::state::CommitState;
use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
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
    return crate::ui::dialog::scrim(
        move || tab.get().map(|tab| tab.hook_open).unwrap_or(false),
        120,
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
            return crate::ui::dialog::modality(
                view()
                .w(px(560.0))
                .h(px(420.0))
                .flex_col()
                .rounded(px(12.0))
                .border_1()
                .border_color(move || design::line())
                .bg(move || design::token("popover"))
                .child(
                    view()
                        .w_full()
                        .h(px(metrics::DIALOG_HEAD_H))
                        .flex_shrink_0()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(14.0))
                        .bg(move || design::panel())
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(
                            text(move || {
                                i18n::t(language_value.get().as_str(), "commit.logTitle")
                            })
                            .text_size(px(crate::ui::BODY))
                            .font_weight(FontWeight(600))
                            .line_height(px(16.0))
                            .text_color(move || design::text()),
                        )
                        .child(view().flex_1())
                        .child(crate::ui::button::button(
                                   
                            move || i18n::t(language_value.get().as_str(), "common.copy"),
                                   crate::ui::button::ButtonVariant::Secondary,
                                   crate::ui::button::ButtonSize::Small,
                                   false,
                                   move || {
                                       crate::shared::os::copy_text(
                                           notice,
                                           language_value,
                                           log_for_copy.clone(),
                                       )
                                   },
                               ))
                        .child(crate::ui::button::button(
                                   
                            move || i18n::t(language_value.get().as_str(), "common.close"),
                                   crate::ui::button::ButtonVariant::Secondary,
                                   crate::ui::button::ButtonSize::Small,
                                   false,
                                   move || close_button(false),
                               )),
                )
                .child(
                    view().w_full().flex_1().min_h_0().p(px(12.0)).child(
                        scroll().w_full().h_full().child(
                            text(log)
                                .text_size(px(crate::ui::CODE))
                                .line_height(px(crate::ui::CODE_LINE))
                                .text_color(move || design::text2()),
                        ),
                    ),
                )
                .into_element(),
            )
            .into_element();
        }),
        move || dismiss(false),
    );
}
