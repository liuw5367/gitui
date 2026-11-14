//! 钩子完整日志弹层（设计板 10 的 `View full log`）。
//!
//! 面板里只放末尾几行做提示，长输出（尤其是失败时的编译日志）放在这里看，
//! 并提供复制与关闭。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
use crate::widgets;
use crate::Actions;
use nsh_core::prelude::*;

/// 日志弹层：居中、可滚动、Esc 或点背景关闭。
pub(crate) fn hook_sheet(
    language: Signal<SharedString>,
    tab: Memo<Option<Tab>>,
    actions: Actions,
) -> impl IntoElement {
    let close = actions.toggle_hook_log.clone();
    let copy_text = actions.copy_text.clone();
    return view()
        .absolute()
        .inset(px(0.0))
        .z_index(120)
        .items_center()
        .justify_center()
        .bg(move || {
            let open = tab.get().map(|tab| tab.hook_open).unwrap_or(false);
            if open {
                design::token("overlay")
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .on_press({
            let close = close.clone();
            move || close(false)
        })
        .child(dynamic(move || -> AnyElement {
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
            let copy_text = copy_text.clone();
            let log_for_copy = log.clone();
            return view()
                .w(px(560.0))
                .h(px(420.0))
                .flex_col()
                .rounded(px(metrics::RADIUS_PANEL))
                .border_1()
                .border_color(move || design::line())
                .bg(move || design::token("popover"))
                .on_press(|| {})
                .child(
                    view()
                        .w_full()
                        .h(px(38.0))
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(14.0))
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(
                            text(move || {
                                i18n::t(language_value.get().as_str(), "commit.logTitle")
                            })
                            .text_size(px(12.5))
                            .font_weight(FontWeight(600))
                            .line_height(px(16.0))
                            .text_color(move || design::text()),
                        )
                        .child(view().flex_1())
                        .child(widgets::small_button(
                            move || i18n::t(language_value.get().as_str(), "common.copy"),
                            move || copy_text(log_for_copy.clone()),
                        ))
                        .child(widgets::small_button(
                            move || i18n::t(language_value.get().as_str(), "common.close"),
                            move || close_button(false),
                        )),
                )
                .child(
                    view().w_full().flex_1().min_h_0().p(px(12.0)).child(
                        scroll().w_full().h_full().child(
                            text(log)
                                .text_size(px(10.0))
                                .line_height(px(15.0))
                                .font_family("SF Mono")
                                .text_color(move || design::text2()),
                        ),
                    ),
                )
                .into_element();
        }))
}
