//! Credentials & tools 分区（设计板 27）：Git credential helper / SSH keys / Tools。
//!
//! Git 凭据由 Git credential helper 管理；本页不连接 GitHub / GitLab 账号。

use super::commands::SettingsCommands;
use crate::design;
use crate::i18n;
use crate::ui;
use crate::ui::select::SelectOption;
use nsh_core::prelude::*;
use std::sync::Arc;

pub(super) fn credentials_page(cmd: SettingsCommands) -> AnyElement {
    let language = cmd.language;
    let settings = cmd.state.settings_state.get_untracked();
    let helper = cmd.state.credential_helper.get_untracked();
    let keys = cmd.state.ssh_keys.get_untracked();
    let helper_text = if helper.trim().is_empty() {
        i18n::t(language.get_untracked().as_str(), "settings.gitHelperNone")
    } else {
        helper
    };
    let helper_row = setting_row(
        language,
        "settings.credentialHelper",
        Some("settings.credentialHelperHint"),
        text(move || helper_text.clone())
            .text_size(px(ui::CODE))
            .line_height(px(ui::CONTROL))
            .font_family(ui::MONO)
            .text_color(move || design::text2())
            .into_element(),
        true,
    );
    let ssh_toggle = ui::check::check_control(
        move || i18n::t(language.get().as_str(), "settings.sshForRemotes"),
        settings.ssh_for_new_remotes,
        false,
        {
            let cmd = cmd.clone();
            let value = settings.ssh_for_new_remotes;
            move |_next: bool| cmd.update_settings(|settings| settings.ssh_for_new_remotes = !value)
        },
    );
    let ssh_row = setting_row(
        language,
        "settings.sshForRemotes",
        Some("settings.sshForRemotesHint"),
        ssh_toggle.into_element(),
        false,
    );
    let mut key_rows = view().w_full().flex_col();
    if keys.is_empty() {
        key_rows = key_rows.child(
            text(move || i18n::t(language.get().as_str(), "settings.sshNoKeys"))
                .py(px(11.0))
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        );
    }
    for entry in &keys {
        let name_text = entry.name.clone();
        let hint_text = format!("{} · {}", entry.fingerprint, entry.path_display);
        let copy_cmd = cmd.clone();
        let copy_name = entry.public_name.clone();
        let actions = view()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .child(ui::button::button(
                move || i18n::t(language.get().as_str(), "settings.copyPublicKey"),
                ui::button::ButtonVariant::Secondary,
                ui::button::ButtonSize::Small,
                false,
                move || copy_cmd.copy_ssh_public_key(copy_name.clone()),
            ));
        key_rows = key_rows.child(
            ui::setting_row::setting_row(
                move || name_text.clone(),
                Some(Box::new(move || hint_text.clone())),
                actions.into_element(),
                true,
            )
            .into_element(),
        );
    }
    let editor = select_control(
        language,
        settings.external_editor.clone(),
        vec![
            ("vscode", "settings.editorVscode"),
            ("cursor", "settings.editorCursor"),
            ("zed", "settings.editorZed"),
            ("terminal", "settings.editorTerminal"),
        ],
        {
            let cmd = cmd.clone();
            Arc::new(move |id| {
                cmd.update_settings(|settings| settings.external_editor = id.to_string())
            })
        },
    );
    let diff = select_control(
        language,
        settings.diff_tool.clone(),
        vec![
            ("builtin", "settings.diffBuiltin"),
            ("filemerge", "settings.diffFileMerge"),
            ("kaleidoscope", "settings.diffKaleidoscope"),
        ],
        {
            let cmd = cmd.clone();
            Arc::new(move |id| cmd.update_settings(|settings| settings.diff_tool = id.to_string()))
        },
    );
    let merge = select_control(
        language,
        settings.merge_tool.clone(),
        vec![
            ("builtin", "settings.mergeBuiltin"),
            ("filemerge", "settings.mergeFileMerge"),
        ],
        {
            let cmd = cmd.clone();
            Arc::new(move |id| cmd.update_settings(|settings| settings.merge_tool = id.to_string()))
        },
    );
    let terminal = select_control(
        language,
        settings.terminal.clone(),
        vec![
            ("terminal", "settings.terminalTerminal"),
            ("iterm", "settings.terminalIterm"),
            ("warp", "settings.terminalWarp"),
        ],
        {
            let cmd = cmd.clone();
            Arc::new(move |id| cmd.update_settings(|settings| settings.terminal = id.to_string()))
        },
    );
    let message_editor = ui::check::check_control(
        move || i18n::t(language.get().as_str(), "settings.editorForMessages"),
        settings.external_editor_for_messages,
        false,
        {
            let cmd = cmd.clone();
            let value = settings.external_editor_for_messages;
            move |_next: bool| {
                cmd.update_settings(|settings| settings.external_editor_for_messages = !value)
            }
        },
    );
    let proxy = select_control(
        language,
        settings.proxy_mode.clone(),
        vec![
            ("system", "settings.proxySystem"),
            ("none", "settings.proxyNone"),
            ("manual", "settings.proxyManual"),
        ],
        {
            let cmd = cmd.clone();
            Arc::new(move |id| cmd.update_settings(|settings| settings.proxy_mode = id.to_string()))
        },
    );
    let verify_ssl = ui::check::check_control(
        move || i18n::t(language.get().as_str(), "settings.verifySsl"),
        settings.verify_ssl,
        false,
        {
            let cmd = cmd.clone();
            let value = settings.verify_ssl;
            move |_next: bool| cmd.update_settings(|settings| settings.verify_ssl = !value)
        },
    );
    let system_git = ui::check::check_control(
        move || i18n::t(language.get().as_str(), "settings.systemGitConfig"),
        settings.use_system_git_config,
        false,
        {
            let cmd = cmd.clone();
            let value = settings.use_system_git_config;
            move |_next: bool| {
                cmd.update_settings(|settings| settings.use_system_git_config = !value)
            }
        },
    );
    return view()
        .w_full()
        .flex_col()
        .child(ui::setting_row::settings_section(
            true,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.gitCredentials")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.gitCredentialsHint")
            })),
            view().w_full().flex_col().child(helper_row).child(ssh_row),
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.sshKeys")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.sshKeysHint")
            })),
            key_rows,
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.tools")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.toolsHint")
            })),
            view()
                .w_full()
                .flex_col()
                .child(setting_row(
                    language,
                    "settings.externalEditor",
                    Some("settings.externalEditorHint"),
                    editor,
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.diffTool",
                    Some("settings.diffToolHint"),
                    diff,
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.mergeTool",
                    Some("settings.mergeToolHint"),
                    merge,
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.terminal",
                    Some("settings.terminalHint"),
                    terminal,
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.editorForMessages",
                    Some("settings.editorForMessagesHint"),
                    message_editor.into_element(),
                    false,
                )),
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.network")
            },
            None,
            view()
                .w_full()
                .flex_col()
                .child(setting_row(
                    language,
                    "settings.proxy",
                    Some("settings.proxyHint"),
                    proxy,
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.verifySsl",
                    Some("settings.verifySslHint"),
                    verify_ssl.into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.systemGitConfig",
                    Some("settings.systemGitConfigHint"),
                    system_git.into_element(),
                    false,
                )),
        ))
        .into_element();
}

fn select_control(
    language: Signal<SharedString>,
    selected: String,
    options: Vec<(&'static str, &'static str)>,
    on_select: Arc<dyn Fn(&'static str)>,
) -> AnyElement {
    let options = options
        .into_iter()
        .map(|(id, key)| {
            SelectOption::new(id, {
                let language = language;
                move || i18n::t(language.get().as_str(), key)
            })
        })
        .collect();
    return ui::select::select(options, selected, Some(210.0), false, move |id| {
        on_select(id)
    })
    .into_element();
}

fn setting_row(
    language: Signal<SharedString>,
    title_key: &'static str,
    hint_key: Option<&'static str>,
    control: AnyElement,
    divider: bool,
) -> AnyElement {
    let hint = hint_key.map(|key| {
        Box::new(move || i18n::t(language.get().as_str(), key)) as Box<dyn Fn() -> String>
    });
    return ui::setting_row::setting_row(
        move || i18n::t(language.get().as_str(), title_key),
        hint,
        control,
        divider,
    )
    .into_element();
}
