//! About 分区（设计板 26）：标识块、真实版本/架构、仓库链接与 Credits。

use crate::brand;
use crate::design;
use crate::i18n;
use crate::ui;
use nsh_core::prelude::*;
use std::sync::Arc;

pub(super) fn about(
    language: Signal<SharedString>,
    open_browser: Arc<dyn Fn(String)>,
) -> AnyElement {
    let arch = if cfg!(target_arch = "aarch64") {
        "settings.appleSilicon"
    } else {
        "settings.intel"
    };
    let version_text = i18n::with_arguments(
        language.get_untracked().as_str(),
        "settings.versionLine",
        &[
            ("version", brand::VERSION.to_string()),
            ("build", brand::BUILD.to_string()),
            ("arch", i18n::t(language.get_untracked().as_str(), arch)),
            (
                "status",
                i18n::t(language.get_untracked().as_str(), "settings.notChecked"),
            ),
        ],
    );
    let links = view()
        .w_full()
        .flex_col()
        .child(link_row(
            language,
            "settings.website",
            Some((brand::REPOSITORY_URL, true)),
            "settings.visit",
            brand::REPOSITORY_URL,
            open_browser.clone(),
            true,
        ))
        .child(link_row(
            language,
            "settings.source",
            Some((brand::REPOSITORY_URL, true)),
            "settings.githubLink",
            brand::REPOSITORY_URL,
            open_browser.clone(),
            true,
        ))
        .child(link_row(
            language,
            "settings.reportIssue",
            Some(("settings.reportIssueHint", false)),
            "settings.issues",
            brand::ISSUES_URL,
            open_browser.clone(),
            true,
        ))
        .child(link_row(
            language,
            "settings.licenses",
            Some(("settings.licensesUnknown", false)),
            "settings.view",
            brand::REPOSITORY_URL,
            open_browser.clone(),
            false,
        ));
    return view()
        .w_full()
        .flex_col()
        .child(
            view()
                .w_full()
                .mt(px(18.0))
                .flex_row()
                .items_center()
                .gap(px(12.0))
                .child(
                    view()
                        .w(px(36.0))
                        .h(px(36.0))
                        .items_center()
                        .justify_center()
                        .rounded(px(9.0))
                        .bg(move || design::accent2())
                        .child(crate::widgets::icon("branch", 18.0, || {
                            design::token("primary-foreground")
                        })),
                )
                .child(
                    view()
                        .flex_col()
                        .child(
                            text(brand::PRODUCT_NAME)
                                .text_size(px(ui::TITLE_MEDIUM))
                                .line_height(px(ui::TITLE_MEDIUM_LINE))
                                .font_weight(FontWeight(700))
                                .letter_spacing(px(-0.4))
                                .text_color(move || design::text()),
                        )
                        .child(
                            text(move || version_text.clone())
                                .mt(px(2.0))
                                .text_size(px(ui::SECONDARY))
                                .line_height(px(ui::SECONDARY_LINE))
                                .text_color(move || design::text3()),
                        ),
                ),
        )
        .child(ui::setting_row::settings_section(
            true,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.links")
            },
            None,
            links,
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.credits")
            },
            None,
            ui::setting_row::setting_row(
                move || i18n::t(language.get().as_str(), "settings.builtWith"),
                Some(Box::new({
                    let language = language;
                    move || i18n::t(language.get().as_str(), "settings.builtWithHint")
                })),
                text(move || i18n::t(language.get().as_str(), "settings.copyright"))
                    .text_size(px(ui::SECONDARY))
                    .line_height(px(ui::SECONDARY_LINE))
                    .text_color(move || design::text3())
                    .into_element(),
                false,
            ),
        ))
        .into_element();
}

fn link_row(
    language: Signal<SharedString>,
    title_key: &'static str,
    hint: Option<(&'static str, bool)>,
    button_key: &'static str,
    url: &'static str,
    open_browser: Arc<dyn Fn(String)>,
    divider: bool,
) -> AnyElement {
    let hint = hint.map(|(key, literal)| {
        Box::new(move || {
            if literal {
                key.to_string()
            } else {
                i18n::t(language.get().as_str(), key)
            }
        }) as Box<dyn Fn() -> String>
    });
    return ui::setting_row::setting_row(
        move || i18n::t(language.get().as_str(), title_key),
        hint,
        ui::button::button(
            move || i18n::t(language.get().as_str(), button_key),
            ui::button::ButtonVariant::Ghost,
            ui::button::ButtonSize::Small,
            false,
            move || open_browser(url.to_string()),
        )
        .into_element(),
        divider,
    )
    .into_element();
}
