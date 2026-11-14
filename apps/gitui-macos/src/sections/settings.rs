//! Settings sheet: General, Git, Credentials, Shortcuts, About.

use crate::design;
use crate::model::short_name;
use gitui_agent::{AgentPermissions, Automation};
use crate::widgets::header_button;
use crate::SettingsTab;
use nsh_base::input;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SettingsActions {
    pub(crate) close: Arc<dyn Fn()>,
    pub(crate) save_identity: Arc<dyn Fn()>,
    pub(crate) open_recent: Arc<dyn Fn(String)>,
    pub(crate) view_audit: Arc<dyn Fn()>,
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

fn shortcut_row(key: &'static str, label: &'static str) -> impl IntoElement {
    view()
        .h(px(30.0))
        .flex_row()
        .items_center()
        .gap_3()
        .border_b_1()
        .border_color(move || design::line_soft())
        .child(
            text(key)
                .w(px(90.0))
                .text_size(px(10.5))
                .text_color(move || design::text()),
        )
        .child(
            text(label)
                .text_size(px(10.5))
                .text_color(move || design::text2()),
        )
}

fn permission_row(
    label: &'static str,
    detail: &'static str,
    enabled: Arc<dyn Fn() -> bool>,
    toggle: Arc<dyn Fn()>,
) -> impl IntoElement {
    let text_enabled = enabled.clone();
    let color_enabled = enabled.clone();
    view()
        .h(px(42.0))
        .flex_row()
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(move || design::line_soft())
        .child(
            view()
                .flex_col()
                .child(
                    text(label)
                        .text_size(px(11.0))
                        .text_color(move || design::text()),
                )
                .child(
                    text(detail)
                        .text_size(px(9.5))
                        .text_color(move || design::text3()),
                ),
        )
        .child(view().flex_1())
        .child(
            view()
                .h(px(22.0))
                .px_2()
                .rounded(px(4.0))
                .bg(move || {
                    if enabled() {
                        design::primary()
                    } else {
                        design::panel2()
                    }
                })
                .on_press(move || toggle())
                .child(
                    text(move || if text_enabled() { "On" } else { "Off" })
                        .text_size(px(10.0))
                        .text_color(move || {
                            if color_enabled() {
                                design::primary_foreground()
                            } else {
                                design::text2()
                            }
                        }),
                ),
        )
}

fn automation_button(
    label: &'static str,
    target: Automation,
    permissions: Signal<AgentPermissions>,
) -> impl IntoElement {
    view()
        .h(px(22.0))
        .px_2()
        .rounded(px(4.0))
        .bg(move || {
            if permissions.get().automation == target {
                design::primary()
            } else {
                design::panel2()
            }
        })
        .on_press(move || {
            let mut next = permissions.get_untracked();
            next.automation = target;
            permissions.set(next);
        })
        .child(
            text(label)
                .text_size(px(10.0))
                .text_color(move || {
                    if permissions.get().automation == target {
                        design::primary_foreground()
                    } else {
                        design::text2()
                    }
                }),
        )
}

pub(crate) fn settings_panel(
    open: Signal<bool>,
    tab: Signal<SettingsTab>,
    identity_name: Signal<String>,
    identity_email: Signal<String>,
    recent: Signal<Vec<SharedString>>,
    permissions: Signal<AgentPermissions>,
    credential_helper: Signal<String>,
    ssh_agent: Signal<bool>,
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
                                .child(nav_item("Agent", SettingsTab::Agent, tab, Arc::new({
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
                                        )
                                        .child(
                                            text("Recent repositories")
                                                .text_size(px(11.0))
                                                .text_color(move || design::text()),
                                        )
                                        .child(each(
                                            recent,
                                            |path: &SharedString| path.clone(),
                                            move |path, _| {
                                                let open = actions.open_recent.clone();
                                                view()
                                                    .h(px(24.0))
                                                    .flex_row()
                                                    .items_center()
                                                    .gap_2()
                                                    .on_press(move || open(path.get().to_string()))
                                                    .child(
                                                        text(move || short_name(&path.get().to_string()))
                                                            .text_size(px(10.5))
                                                            .text_color(move || design::text2()),
                                                    )
                                            },
                                        )),
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
                                            if tab.get() == SettingsTab::Agent {
                                                px(420.0)
                                            } else {
                                                px(0.0)
                                            }
                                        })
                                        .flex_col()
                                        .gap_1()
                                        .child(section("Agent", "Permissions for the future agent runtime."))
                                        .child(
                                            text("Agent provider is not connected; manual plan approval is available. Policy is session-only.")
                                                .text_size(px(10.0))
                                                .text_color(move || design::text3()),
                                        )
                                        .child(
                                            text("Audit: ~/Library/Application Support/gitui/agent-audit.log")
                                                .text_size(px(9.5))
                                                .text_color(move || design::text3()),
                                        )
                                        .child(header_button("View audit log", false, {
                                            let action = actions.view_audit.clone();
                                            move || action()
                                        }))
                                        .child(
                                            view()
                                                .h(px(38.0))
                                                .flex_row()
                                                .items_center()
                                                .gap_2()
                                                .border_b_1()
                                                .border_color(move || design::line_soft())
                                                .child(
                                                    text("Read repository")
                                                        .flex_1()
                                                        .text_size(px(11.0))
                                                        .text_color(move || design::text()),
                                                )
                                                .child(
                                                    text("On")
                                                        .text_size(px(10.0))
                                                        .text_color(move || design::text3()),
                                                ),
                                        )
                                        .child(permission_row(
                                            "Stage files",
                                            "Move working-tree changes into the Git index.",
                                            Arc::new(move || permissions.get().stage),
                                            Arc::new(move || {
                                                let mut next = permissions.get_untracked();
                                                next.stage = !next.stage;
                                                permissions.set(next);
                                            }),
                                        ))
                                        .child(permission_row(
                                            "Create commits",
                                            "Commit staged changes on the current branch.",
                                            Arc::new(move || permissions.get().commit),
                                            Arc::new(move || {
                                                let mut next = permissions.get_untracked();
                                                next.commit = !next.commit;
                                                permissions.set(next);
                                            }),
                                        ))
                                        .child(permission_row(
                                            "Push to remote",
                                            "Send commits to the configured upstream.",
                                            Arc::new(move || permissions.get().push),
                                            Arc::new(move || {
                                                let mut next = permissions.get_untracked();
                                                next.push = !next.push;
                                                permissions.set(next);
                                            }),
                                        ))
                                        .child(permission_row(
                                            "Force push",
                                            "Overwrite remote history. Not recommended.",
                                            Arc::new(move || permissions.get().force_push),
                                            Arc::new(move || {
                                                let mut next = permissions.get_untracked();
                                                next.force_push = !next.force_push;
                                                permissions.set(next);
                                            }),
                                        ))
                                        .child(permission_row(
                                            "Discard changes",
                                            "Permanently delete uncommitted changes.",
                                            Arc::new(move || permissions.get().discard),
                                            Arc::new(move || {
                                                let mut next = permissions.get_untracked();
                                                next.discard = !next.discard;
                                                permissions.set(next);
                                            }),
                                        ))
                                        .child(
                                            view()
                                                .flex_row()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    text("Automation")
                                                        .flex_1()
                                                        .text_size(px(11.0))
                                                        .text_color(move || design::text()),
                                                )
                                                .child(automation_button("Off", Automation::Off, permissions))
                                                .child(automation_button("Commit", Automation::Commit, permissions))
                                                .child(automation_button(
                                                    "Commit & Push",
                                                    Automation::CommitAndPush,
                                                    permissions,
                                                )),
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
                                            text(move || {
                                                if credential_helper.get().is_empty() {
                                                    "credential.helper: not configured".to_string()
                                                } else {
                                                    format!("credential.helper: {}", credential_helper.get())
                                                }
                                            })
                                            .text_size(px(10.5))
                                            .text_color(move || design::text2()),
                                        )
                                        .child(
                                            text("SSH")
                                                .text_size(px(11.0))
                                                .text_color(move || design::text()),
                                        )
                                        .child(
                                            text(move || {
                                                if ssh_agent.get() {
                                                    "SSH_AUTH_SOCK: available · keys stay in the system agent"
                                                        .to_string()
                                                } else {
                                                    "SSH_AUTH_SOCK: not set · SSH may prompt"
                                                        .to_string()
                                                }
                                            })
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
                                        .child(shortcut_row("⌘K", "Open command palette"))
                                        .child(shortcut_row("Esc", "Close command palette"))
                                        .child(shortcut_row("↑ / ↓", "Move through commands"))
                                        .child(shortcut_row("Enter", "Run selected command"))
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
