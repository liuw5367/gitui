//! Ed25519 SSH key generation form.

use super::commands::SettingsCommands;
use super::state::SettingsState;
use crate::design;
use crate::i18n;
use crate::ui;
use nsh_core::prelude::*;

pub(super) fn ssh_generate_dialog(
    state: &SettingsState,
    cmd: &SettingsCommands,
) -> impl IntoElement + use<> {
    let form_state = state.ssh_generate;
    let busy = state.ssh_operation;
    let language = cmd.language;
    let dismiss = cmd.clone();
    let footer_command = cmd.clone();
    return dynamic(move || -> AnyElement {
        let Some(form) = form_state.get() else {
            return view().w(px(0.0)).h(px(0.0)).into_element();
        };
        let name = form.name;
        let comment = form.comment;
        let body_name = name;
        let body_comment = comment;
        let feedback_form = form.clone();
        let operation_running = busy.get();
        let mut body = view().w_full().flex_col().gap(px(ui::GAP)).child(
            text(move || i18n::t(language.get().as_str(), "settings.sshGenerateHint"))
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text2()),
        );
        if operation_running {
            body = body.child(
                text(move || i18n::t(language.get().as_str(), "settings.sshGenerating"))
                    .text_size(px(ui::SECONDARY))
                    .line_height(px(ui::SECONDARY_LINE))
                    .text_color(move || design::text2()),
            );
        } else {
            body = body
                .child(
                    view()
                        .w_full()
                        .flex_col()
                        .gap(px(4.0))
                        .child(field_label(language, "settings.sshKeyName"))
                        .child(ui::text_input::text_input(
                            body_name,
                            move || {
                                i18n::t(language.get().as_str(), "settings.sshKeyNamePlaceholder")
                            },
                            false,
                            move |value| body_name.set(value.to_string()),
                        )),
                )
                .child(
                    view()
                        .w_full()
                        .flex_col()
                        .gap(px(4.0))
                        .child(field_label(language, "settings.sshKeyComment"))
                        .child(ui::text_input::text_input(
                            body_comment,
                            move || {
                                i18n::t(
                                    language.get().as_str(),
                                    "settings.sshKeyCommentPlaceholder",
                                )
                            },
                            false,
                            move |value| body_comment.set(value.to_string()),
                        )),
                );
        }
        body = body.child(dynamic(move || -> AnyElement {
            let Some(error) = feedback_form.error.get() else {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            };
            return text(move || error.clone())
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::danger())
                .into_element();
        }));

        let footer_form = form.clone();
        let footer_name = name;
        let footer_cmd = footer_command.clone();
        let footer = dynamic(move || -> AnyElement {
            let operation_running = busy.get();
            let name_is_empty = footer_name.get().trim().is_empty();
            let cancel_cmd = footer_cmd.clone();
            let submit_cmd = footer_cmd.clone();
            return view()
                .flex_row()
                .items_center()
                .justify_end()
                .gap(px(ui::GAP))
                .child(ui::button::button(
                    move || i18n::t(language.get().as_str(), "common.cancel"),
                    ui::button::ButtonVariant::Secondary,
                    ui::button::ButtonSize::Regular,
                    operation_running,
                    move || cancel_cmd.close_ssh_generate(),
                ))
                .child(ui::button::button(
                    move || i18n::t(language.get().as_str(), "settings.sshGenerateAction"),
                    ui::button::ButtonVariant::Primary,
                    ui::button::ButtonSize::Regular,
                    operation_running || name_is_empty,
                    move || {
                        footer_form.error.set(None);
                        submit_cmd.submit_ssh_generate();
                    },
                ))
                .into_element();
        });
        let dismiss_cmd = dismiss.clone();
        return ui::dialog::dialog(
            true,
            520.0,
            move || i18n::t(language.get().as_str(), "settings.sshGenerateTitle"),
            body,
            vec![footer.into_element()],
            move || dismiss_cmd.close_ssh_generate(),
        );
    });
}

fn field_label(language: Signal<SharedString>, key: &'static str) -> AnyElement {
    return text(move || i18n::t(language.get().as_str(), key))
        .text_size(px(ui::SECONDARY))
        .line_height(px(ui::SECONDARY_LINE))
        .text_color(move || design::text2())
        .into_element();
}
