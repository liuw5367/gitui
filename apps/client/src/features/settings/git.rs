//! 设置页的 Git 分区（设计板 24）：本机 Git 版本与执行路径、凭据助手（只读）
//! 与仓库发现。

use super::view::{section_block, toggle_row};
use crate::design::{self, metrics};
use crate::i18n;
use nsh_core::prelude::*;
use std::sync::Arc;

/// Git 分区（24）：本机 Git 版本与执行路径、凭据助手（只读）、仓库发现（未实现）。
#[allow(clippy::too_many_arguments)]
pub(super) fn git_page(
    language: Signal<SharedString>,
    info: (String, String, String),
    choose: Arc<dyn Fn()>,
    detect: Arc<dyn Fn()>,
    toggle_path: Arc<dyn Fn()>,
    use_path: bool,
    scan_folders: Vec<String>,
    found: usize,
    add_found: Arc<dyn Fn()>,
) -> AnyElement {
    let (version, exec_path, helper) = info;
    let helper_text = if helper.trim().is_empty() {
        i18n::t(language.get_untracked().as_str(), "settings.gitHelperNone")
    } else {
        helper
    };
    return view()
        .w_full()
        .flex_col()
        .gap(px(18.0))
        .child(section_block(
            language,
            "settings.gitExecutable",
            "settings.gitExecutableHint",
            view()
                .flex_col()
                .gap(px(4.0))
                .child(
                    text(version)
                        .text_size(px(crate::ui::CODE))
                        .line_height(px(15.0))
                        .text_color(move || design::text()),
                )
                .child(
                    text(exec_path)
                        .text_size(px(crate::ui::CODE))
                        .line_height(px(14.0))
                        .text_color(move || design::text3())
                        .text_overflow(nsh_core::EllipsizeMode::Middle),
                )
                .child(
                    view()
                        .w_full()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(crate::ui::button::button(
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "settings.choose")
                            },
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            false,
                            {
                                let choose = choose.clone();
                                move || choose()
                            },
                        ))
                        .child(crate::ui::button::button(
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "settings.detect")
                            },
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            false,
                            {
                                let detect = detect.clone();
                                move || detect()
                            },
                        ))
                        .child(toggle_row(
                            language,
                            "settings.usePath",
                            Arc::new(move || use_path),
                            {
                                let toggle_path = toggle_path.clone();
                                move || toggle_path()
                            },
                        )),
                )
                .into_element(),
        ))
        .child(section_block(
            language,
            "settings.discovery2",
            "settings.discovery2Hint",
            view()
                .w_full()
                .flex_col()
                .gap(px(6.0))
                .child(
                    view().w_full().flex_col().gap(px(2.0)).child(
                        scan_folders
                            .iter()
                            .map(|folder| {
                                let label = folder.clone();
                                return view()
                                    .w_full()
                                    .h(px(22.0))
                                    .px(px(7.0))
                                    .items_center()
                                    .rounded(px(4.0))
                                    .bg(move || design::panel2())
                                    .child(
                                        text(label)
                                            .text_size(px(crate::ui::CODE))
                                            .line_height(px(crate::ui::CODE_LINE))
                                            .text_color(move || design::text2())
                                            .text_overflow(nsh_core::EllipsizeMode::Middle),
                                    )
                                    .into_element();
                            })
                            .collect::<Vec<AnyElement>>(),
                    ),
                )
                .child(
                    view()
                        .w_full()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            text(move || {
                                i18n::with_arguments(
                                    language.get().as_str(),
                                    "settings.scanFound",
                                    &[("count", found.to_string())],
                                )
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .line_height(px(crate::ui::SECONDARY_LINE))
                            .text_color(move || design::text3()),
                        )
                        .child(view().flex_1())
                        .child(crate::ui::button::button(
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "settings.scanNow")
                            },
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            false,
                            {
                                let add = add_found.clone();
                                move || add()
                            },
                        )),
                )
                .into_element(),
        ))
        .child(section_block(
            language,
            "settings.credentialHelper",
            "settings.credentialHelperHint",
            view()
                .flex_col()
                .gap(px(4.0))
                .child(
                    text(helper_text)
                        .text_size(px(crate::ui::CODE))
                        .line_height(px(15.0))
                        .text_color(move || design::text2()),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "settings.gitHelperNote"))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .into_element(),
        ))
        .child(section_block(
            language,
            "settings.discovery",
            "settings.discoveryHint",
            disabled_row(language, "settings.scanNow"),
        ))
        .into_element();
}

/// 禁用态按钮 + 「尚未实现」说明。
fn disabled_row(language: Signal<SharedString>, label_key: &'static str) -> AnyElement {
    return view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            view()
                .h(px(metrics::BUTTON_SMALL_H))
                .px(px(8.0))
                .items_center()
                .rounded(px(5.0))
                .border_1()
                .border_color(move || design::btn_border())
                .bg(move || design::btn())
                .opacity(0.45)
                .child(
                    text(move || i18n::t(language.get().as_str(), label_key))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(14.0))
                        .text_color(move || design::text()),
                ),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), "settings.notImplemented"))
                .text_size(px(crate::ui::META))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .into_element();
}
