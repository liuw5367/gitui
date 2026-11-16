//! 设置页的 About 与 Credentials 分区（设计板 26 / 27）：版本、外链、凭据助手与
//! `~/.ssh` 状态（只读）。

use crate::brand;
use crate::design;
use crate::i18n;
use nsh_core::prelude::*;
use std::sync::Arc;

pub(super) fn about(
    language: Signal<SharedString>,
    name: String,
    email: String,
    open_browser: Arc<dyn Fn(String)>,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(8.0))
        .child(
            text(brand::version_line())
                .text_size(px(crate::ui::CODE))
                .text_color(move || design::text2()),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), "settings.aboutHint"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .child(
            text(format!("{} <{}>", name, email))
                .text_size(px(crate::ui::CODE))
                .text_color(move || design::text2()),
        )
        // 四个外链；打不开时走状态栏提示，不弹错误框。
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .flex_wrap()
                .gap(px(6.0))
                .child(link_button(
                    language,
                    "settings.website",
                    "https://example.com/git-client",
                    open_browser.clone(),
                ))
                .child(link_button(
                    language,
                    "settings.source",
                    "https://example.com/git-client/source",
                    open_browser.clone(),
                ))
                .child(link_button(
                    language,
                    "settings.reportIssue",
                    "https://example.com/git-client/issues",
                    open_browser.clone(),
                ))
                .child(link_button(
                    language,
                    "settings.licenses",
                    "https://example.com/git-client/licenses",
                    open_browser.clone(),
                )),
        );
}

/// 一个外链按钮。
fn link_button(
    language: Signal<SharedString>,
    key: &'static str,
    url: &'static str,
    open_browser: Arc<dyn Fn(String)>,
) -> impl IntoElement {
    return crate::ui::button::button(
        move || i18n::t(language.get().as_str(), key),
        crate::ui::button::ButtonVariant::Secondary,
        crate::ui::button::ButtonSize::Small,
        false,
        move || open_browser(url.to_string()),
    );
}

/// 凭据分区（27）：本机 Git 的凭据助手与 ~/.ssh 状态（只读）。
pub(super) fn credentials_page(
    language: Signal<SharedString>,
    info: (String, String, String),
    notice: Signal<Option<String>>,
) -> AnyElement {
    let (_, _, helper) = info;
    let helper_text = if helper.trim().is_empty() {
        i18n::t(language.get_untracked().as_str(), "settings.gitHelperNone")
    } else {
        helper
    };
    let keys = std::env::var_os("HOME")
        .map(|home| std::path::PathBuf::from(home).join(".ssh"))
        .map(|dir| {
            std::fs::read_dir(dir)
                .map(|entries| entries.count())
                .unwrap_or(0)
        })
        .unwrap_or(0);
    return view()
        .w_full()
        .flex_col()
        .gap(px(18.0))
        .child(super::view::section_block(
            language,
            "settings.connections",
            "settings.connectionsHint",
            view()
                .flex_col()
                .gap(px(4.0))
                .child(
                    text(helper_text)
                        .text_size(px(crate::ui::CODE))
                        .line_height(px(15.0))
                        .text_color(move || design::text()),
                )
                .child(pending_row(language, "settings.signIn", notice))
                .into_element(),
        ))
        .child(super::view::section_block(
            language,
            "settings.sshKeys",
            "settings.sshKeysHint",
            view()
                .flex_col()
                .gap(px(4.0))
                .child(
                    text(move || {
                        i18n::with_arguments(
                            language.get().as_str(),
                            "settings.sshKeyCount",
                            &[("count", keys.to_string())],
                        )
                    })
                    .text_size(px(crate::ui::SECONDARY))
                    .line_height(px(15.0))
                    .text_color(move || design::text2()),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "settings.sshAgentsNote"))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .into_element(),
        ))
        .into_element();
}

/// 可点但尚未接入的按钮：点一下在状态栏说明原因，不写假数据。
fn pending_row(
    language: Signal<SharedString>,
    label_key: &'static str,
    notice: Signal<Option<String>>,
) -> AnyElement {
    return view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(crate::ui::button::button(
            move || i18n::t(language.get().as_str(), label_key),
            crate::ui::button::ButtonVariant::Secondary,
            crate::ui::button::ButtonSize::Small,
            false,
            move || {
                let lang = language.get_untracked();
                crate::interactions::notify(
                    notice,
                    i18n::t(lang.as_str(), "settings.credentialsPending"),
                );
            },
        ))
        .child(
            text(move || i18n::t(language.get().as_str(), "settings.notConnected"))
                .text_size(px(crate::ui::META))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .into_element();
}
