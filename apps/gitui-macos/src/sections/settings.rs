//! Settings sheet: General, Git, Credentials, Shortcuts, About.

use crate::design;
use crate::widgets::header_button;
use crate::SettingsTab;
use nsh_base::input;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SettingsActions {
    pub(crate) close: Arc<dyn Fn()>,
    pub(crate) save_identity: Arc<dyn Fn()>,
}

fn nav_item(
    label: &'static str,
    target: SettingsTab,
    tab: Signal<SettingsTab>,
    on_tab: Arc<dyn Fn(SettingsTab)>,
) -> impl IntoElement {
    view()
        .h(px(28.0))
        .px_2()
        .rounded(px(6.0))
        .items_center()
        .bg(move || {
            if tab.get() == target {
                design::primary()
            } else {
                design::sidebar()
            }
        })
        .on_press(move || on_tab(target))
        .child(
            text(label)
                .text_size(px(11.5))
                .text_color(move || {
                    if tab.get() == target {
                        design::primary_foreground()
                    } else {
                        design::text()
                    }
                }),
        )
}

fn section(title: &'static str, subtitle: &'static str) -> impl IntoElement {
    view()
        .flex_col()
        .gap_2()
        .child(
            text(title)
                .text_size(px(16.0))
                .text_color(move || design::text()),
        )
        .child(
            text(subtitle)
                .text_size(px(10.5))
                .text_color(move || design::text3()),
        )
}

pub(crate) fn settings_panel(
    open: Signal<bool>,
    tab: Signal<SettingsTab>,
    identity_name: Signal<String>,
    identity_email: Signal<String>,
    actions: SettingsActions,
) -> impl IntoElement {
    view()
        .absolute()
        .inset(px(0.0))
        .w_full()
        .h_full()
        .z_index(move || if open.get() { 40 } else { -1 })
        .items_center()
        .justify_center()
        .p(px(24.0))
        .bg(move || {
            if open.get() {
                AdaptiveColor::from(rgb(0x000000)).opacity(0.28)
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .child(
            view()
                .w(px(900.0))
                .h(px(620.0))
                .flex_col()
                .rounded(px(12.0))
                .bg(move || design::content())
                .border_1()
                .border_color(move || design::line())
                .overflow_hidden()
                .child(
                    view()
                        .h(px(44.0))
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .bg(move || design::panel())
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(
                            text("Settings")
                                .flex_1()
                                .text_size(px(13.0))
                                .text_color(move || design::text()),
                        )
                        .child(header_button("Done", true, {
                            let action = actions.close.clone();
                            move || action()
                        })),
                )
                .child(
                    view()
                        .flex_1()
                        .min_h_0()
                        .flex_row()
                        .child(
                            view()
                                .w(px(200.0))
                                .flex_col()
                                .gap_1()
                                .p_2()
                                .bg(move || design::sidebar())
                                .border_r_1()
                                .border_color(move || design::line())
                                .child(
                                    text("gitui")
                                        .text_size(px(13.0))
                                        .text_color(move || design::text()),
                                )
                                .child(
                                    text("Version 0.1.0")
                                        .text_size(px(10.0))
                                        .text_color(move || design::text3()),
                                )
                                .child(view().h(px(8.0)))
                                .child(nav_item("General", SettingsTab::General, tab, Arc::new({
                                    let on_tab = Arc::new(move |next| tab.set(next));
                                    move |next| on_tab(next)
                                })))
                                .child(nav_item("Git", SettingsTab::Git, tab, Arc::new({
                                    let on_tab = Arc::new(move |next| tab.set(next));
                                    move |next| on_tab(next)
                                })))
                                .child(nav_item("Credentials", SettingsTab::Credentials, tab, Arc::new({
                                    let on_tab = Arc::new(move |next| tab.set(next));
                                    move |next| on_tab(next)
                                })))
                                .child(nav_item("Shortcuts", SettingsTab::Shortcuts, tab, Arc::new({
                                    let on_tab = Arc::new(move |next| tab.set(next));
                                    move |next| on_tab(next)
                                })))
                                .child(nav_item("About gitui", SettingsTab::About, tab, Arc::new({
                                    let on_tab = Arc::new(move |next| tab.set(next));
                                    move |next| on_tab(next)
                                }))),
                        )
                        .child(
                            view()
                                .flex_1()
                                .min_h_0()
                                .flex_col()
                                .p_4()
                                .gap_2()
                                .child(
                                    view()
                                        .h(move || {
                                            if tab.get() == SettingsTab::General {
                                                px(480.0)
                                            } else {
                                                px(0.0)
                                            }
                                        })
                                        .flex_col()
                                        .gap_2()
                                        .child(section("General", "Application behavior."))
                                        .child(
                                            text("Appearance follows the macOS system setting.")
                                                .text_size(px(10.5))
                                                .text_color(move || design::text2()),
                                        )
                                        .child(
                                            text("Repository scanning is manual; open a path from the toolbar.")
                                                .text_size(px(10.5))
                                                .text_color(move || design::text3()),
                                        ),
                                )
                                .child(
                                    view()
                                        .h(move || {
                                            if tab.get() == SettingsTab::Git {
                                                px(480.0)
                                            } else {
                                                px(0.0)
                                            }
                                        })
                                        .flex_col()
                                        .gap_2()
                                        .child(section("Git", "Executable, repository discovery, and identity."))
                                        .child(
                                            text("Commit identity")
                                                .text_size(px(11.0))
                                                .text_color(move || design::text()),
                                        )
                                        .child(
                                            input()
                                                .value(identity_name)
                                                .on_change(move |text: &str| identity_name.set(text.to_string()))
                                                .placeholder("user.name")
                                                .text_size(px(11.0)),
                                        )
                                        .child(
                                            input()
                                                .value(identity_email)
                                                .on_change(move |text: &str| identity_email.set(text.to_string()))
                                                .placeholder("user.email")
                                                .text_size(px(11.0)),
                                        )
                                        .child(header_button("Save identity", true, {
                                            let action = actions.save_identity.clone();
                                            move || action()
                                        }))
                                        .child(
                                            text("Hooks run from .git/hooks. Use No hooks in the commit inspector to bypass once.")
                                                .text_size(px(10.0))
                                                .text_color(move || design::text3()),
                                        ),
                                )
                                .child(
                                    view()
                                        .h(move || {
                                            if tab.get() == SettingsTab::Credentials {
                                                px(480.0)
                                            } else {
                                                px(0.0)
                                            }
                                        })
                                        .flex_col()
                                        .gap_2()
                                        .child(section("Credentials", "Secrets stay with the system."))
                                        .child(
                                            text("Git credential helper")
                                                .text_size(px(11.0))
                                                .text_color(move || design::text()),
                                        )
                                        .child(
                                            text("HTTPS credentials are resolved by Git's configured helper.")
                                                .text_size(px(10.5))
                                                .text_color(move || design::text2()),
                                        )
                                        .child(
                                            text("SSH")
                                                .text_size(px(11.0))
                                                .text_color(move || design::text()),
                                        )
                                        .child(
                                            text("SSH keys and agent are resolved by the system; gitui does not store them.")
                                                .text_size(px(10.5))
                                                .text_color(move || design::text2()),
                                        ),
                                )
                                .child(
                                    view()
                                        .h(move || {
                                            if tab.get() == SettingsTab::Shortcuts {
                                                px(480.0)
                                            } else {
                                                px(0.0)
                                            }
                                        })
                                        .flex_col()
                                        .gap_2()
                                        .child(section("Shortcuts", "Keyboard navigation."))
                                        .child(
                                            text("Shortcut customization is not available yet.")
                                                .text_size(px(10.5))
                                                .text_color(move || design::text3()),
                                        ),
                                )
                                .child(
                                    view()
                                        .h(move || {
                                            if tab.get() == SettingsTab::About {
                                                px(480.0)
                                            } else {
                                                px(0.0)
                                            }
                                        })
                                        .flex_col()
                                        .gap_2()
                                        .child(section("About gitui", "Native Git for macOS."))
                                        .child(
                                            text("gitui 0.1.0 · NSH + Rust")
                                                .text_size(px(11.0))
                                                .text_color(move || design::text()),
                                        )
                                        .child(
                                            text("Git engine: system git CLI")
                                                .text_size(px(10.5))
                                                .text_color(move || design::text2()),
                                        ),
                                ),
                        ),
                ),
        )
}
