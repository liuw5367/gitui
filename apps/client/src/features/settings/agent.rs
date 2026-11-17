//! Agent 分区（设计板 23b）：Provider protocol / Endpoint / Model / Privacy 四段十二行。
//!
//! 本构建没有 Agent provider 运行时；App key 只保存在本次会话的内存信号里，
//! 协议、端点、模型与隐私偏好写入 `store::Settings`，不发起网络请求。

use super::commands::SettingsCommands;
use crate::design;
use crate::i18n;
use crate::ui;
use crate::ui::select::SelectOption;
use nsh_core::prelude::*;

const OPENAI_MODELS: [&str; 3] = ["gpt-4.1", "gpt-4o-mini", "o4-mini"];
const ANTHROPIC_MODELS: [&str; 3] = [
    "claude-sonnet-4-5",
    "claude-opus-4-1",
    "claude-haiku-4-5",
];

pub(super) fn agent_page(cmd: SettingsCommands) -> AnyElement {
    let language = cmd.language;
    let settings = cmd.state.settings_state.get_untracked();
    let base_url = cmd.state.agent_base_url;
    let app_key = cmd.state.agent_app_key;
    let temperature = cmd.state.agent_temperature;
    let max_tokens = cmd.state.agent_max_tokens;
    let protocol = settings.agent_protocol.clone();
    let resolved = resolved_endpoint(settings.agent_base_url.as_str(), protocol.as_str());
    let wire = wire_format(protocol.as_str()).to_string();
    let models = model_ids(protocol.as_str());
    let protocol_options = ui::segmented::segmented(
        vec![
            ui::segmented::Segment {
                id: "openai",
                label: Box::new({
                    let language = language;
                    move || i18n::t(language.get().as_str(), "settings.agentOpenAI")
                }),
            },
            ui::segmented::Segment {
                id: "anthropic",
                label: Box::new({
                    let language = language;
                    move || i18n::t(language.get().as_str(), "settings.agentAnthropic")
                }),
            },
        ],
        protocol.clone(),
        {
            let cmd = cmd.clone();
            move |id| {
                let models = model_ids(id);
                cmd.update_settings(|settings| {
                    settings.agent_protocol = id.to_string();
                    if !models.contains(&settings.agent_model.as_str()) {
                        settings.agent_model = models[0].to_string();
                    }
                });
            }
        },
    );
    let base_field = ui::text_input::text_input_bound_commit(
        move || base_url.get(),
        move || i18n::t(language.get().as_str(), "settings.agentBaseUrlPlaceholder"),
        false,
        false,
        {
            let base_url = base_url;
            move |text: String| base_url.set(text)
        },
        {
            let cmd = cmd.clone();
            let base_url = base_url;
            move || {
                let value = base_url.get_untracked().trim().trim_end_matches('/').to_string();
                cmd.update_settings(|settings| settings.agent_base_url = value.clone());
                base_url.set(value);
            }
        },
    );
    let key_field = ui::text_input::masked_input(
        app_key,
        move || i18n::t(language.get().as_str(), "settings.agentAppKeyPlaceholder"),
        move |text: String| app_key.set(text),
    );
    let temperature_field = ui::text_input::text_input_bound_commit(
        move || temperature.get(),
        move || "0.2".to_string(),
        false,
        false,
        {
            let temperature = temperature;
            move |text: String| temperature.set(text)
        },
        {
            let cmd = cmd.clone();
            let temperature = temperature;
            move || {
                let raw = temperature.get_untracked();
                let Ok(value) = raw.trim().parse::<f32>() else {
                    let fallback = cmd
                        .state
                        .settings_state
                        .get_untracked()
                        .agent_temperature
                        .to_string();
                    temperature.set(fallback);
                    return;
                };
                let value = value.clamp(0.0, 2.0);
                cmd.update_settings(|settings| settings.agent_temperature = value);
                temperature.set(value.to_string());
            }
        },
    );
    let max_tokens_field = ui::text_input::text_input_bound_commit(
        move || max_tokens.get(),
        move || "4096".to_string(),
        false,
        false,
        {
            let max_tokens = max_tokens;
            move |text: String| max_tokens.set(text)
        },
        {
            let cmd = cmd.clone();
            let max_tokens = max_tokens;
            move || {
                let raw = max_tokens.get_untracked();
                let Ok(value) = raw.trim().parse::<u32>() else {
                    let fallback = cmd
                        .state
                        .settings_state
                        .get_untracked()
                        .agent_max_tokens
                        .to_string();
                    max_tokens.set(fallback);
                    return;
                };
                let value = value.clamp(1, 32_768);
                cmd.update_settings(|settings| settings.agent_max_tokens = value);
                max_tokens.set(value.to_string());
            }
        },
    );
    let model_options = models
        .iter()
        .map(|id| {
            let id = *id;
            SelectOption::new(id, {
                let language = language;
                move || model_label(language, id)
            })
        })
        .collect::<Vec<SelectOption>>();
    let model_select = ui::select::select(
        model_options,
        settings.agent_model.clone(),
        Some(210.0),
        {
            let cmd = cmd.clone();
            move |id| cmd.update_settings(|settings| settings.agent_model = id.to_string())
        },
    );
    let send_diffs = ui::check::check_control(
        move || i18n::t(language.get().as_str(), "settings.agentSendDiffs"),
        settings.agent_send_diffs,
        false,
        {
            let cmd = cmd.clone();
            let value = settings.agent_send_diffs;
            move |_next: bool| {
                cmd.update_settings(|settings| settings.agent_send_diffs = !value)
            }
        },
    );
    let send_path = ui::check::check_control(
        move || i18n::t(language.get().as_str(), "settings.agentSendRepositoryPath"),
        settings.agent_send_repository_path,
        false,
        {
            let cmd = cmd.clone();
            let value = settings.agent_send_repository_path;
            move |_next: bool| {
                cmd.update_settings(|settings| settings.agent_send_repository_path = !value)
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
                move || i18n::t(language.get().as_str(), "settings.agentProvider")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.agentProviderHint")
            })),
            view()
                .w_full()
                .flex_col()
                .child(setting_row(
                    language,
                    "settings.agentProtocol",
                    Some("settings.agentProtocolHint"),
                    protocol_options.into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.agentWire",
                    Some("settings.agentWireHint"),
                    mono(wire, language),
                    false,
                )),
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.agentEndpoint")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.agentEndpointHint")
            })),
            view()
                .w_full()
                .flex_col()
                .child(setting_row(
                    language,
                    "settings.agentBaseUrl",
                    Some("settings.agentBaseUrlHint"),
                    view().w(px(280.0)).flex_shrink_0().child(base_field).into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.agentResolved",
                    Some("settings.agentResolvedHint"),
                    mono(resolved, language),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.agentAppKey",
                    Some("settings.agentAppKeyHint"),
                    view().w(px(280.0)).flex_shrink_0().child(key_field).into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.agentConnection",
                    Some("settings.agentConnectionHint"),
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(ui::button::button(
                            move || i18n::t(language.get().as_str(), "settings.agentTest"),
                            ui::button::ButtonVariant::Secondary,
                            ui::button::ButtonSize::Small,
                            false,
                            {
                                let cmd = cmd.clone();
                                move || cmd.test_agent_connection()
                            },
                        ))
                        .child(
                            text(move || {
                                i18n::t(language.get().as_str(), "settings.agentNotTested")
                            })
                            .text_size(px(ui::SECONDARY))
                            .text_color(move || design::text3()),
                        )
                        .into_element(),
                    false,
                )),
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.agentModelSection")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.agentModelSectionHint")
            })),
            view()
                .w_full()
                .flex_col()
                .child(setting_row(
                    language,
                    "settings.agentModel",
                    Some("settings.agentModelHint"),
                    model_select.into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.agentTemperature",
                    Some("settings.agentTemperatureHint"),
                    view()
                        .w(px(80.0))
                        .flex_shrink_0()
                        .child(temperature_field)
                        .into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.agentMaxTokens",
                    Some("settings.agentMaxTokensHint"),
                    view()
                        .w(px(90.0))
                        .flex_shrink_0()
                        .child(max_tokens_field)
                        .into_element(),
                    false,
                )),
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.agentPrivacy")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.agentPrivacyHint")
            })),
            view()
                .w_full()
                .flex_col()
                .child(setting_row(
                    language,
                    "settings.agentSendDiffs",
                    Some("settings.agentSendDiffsHint"),
                    send_diffs.into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.agentSendRepositoryPath",
                    Some("settings.agentSendRepositoryPathHint"),
                    send_path.into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.agentLocalLog",
                    Some("settings.agentLocalLogHint"),
                    ui::button::button(
                        move || i18n::t(language.get().as_str(), "settings.agentOpenLog"),
                        ui::button::ButtonVariant::Secondary,
                        ui::button::ButtonSize::Small,
                        false,
                        {
                            let cmd = cmd.clone();
                            move || cmd.open_agent_log()
                        },
                    )
                    .into_element(),
                    false,
                )),
        ))
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

fn mono(text_value: String, language: Signal<SharedString>) -> AnyElement {
    return text(move || {
        if text_value.is_empty() {
            i18n::t(language.get().as_str(), "settings.agentResolvedEmpty")
        } else {
            text_value.clone()
        }
    })
    .text_size(px(ui::SECONDARY))
    .line_height(px(ui::SECONDARY_LINE))
    .font_family(ui::MONO)
    .text_color(move || design::text2())
    .into_element();
}

fn wire_format(protocol: &str) -> &'static str {
    return match protocol {
        "anthropic" => "POST /v1/messages",
        _ => "POST /v1/chat/completions",
    };
}

fn resolved_endpoint(base: &str, protocol: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    if base.is_empty() {
        return String::new();
    }
    return format!("{base}{}", wire_format(protocol).trim_start_matches("POST "));
}

fn model_ids(protocol: &str) -> &'static [&'static str] {
    return match protocol {
        "anthropic" => &ANTHROPIC_MODELS,
        _ => &OPENAI_MODELS,
    };
}

fn model_label(language: Signal<SharedString>, id: &'static str) -> String {
    let key = match id {
        "gpt-4.1" => "settings.modelGpt41",
        "gpt-4o-mini" => "settings.modelGpt4oMini",
        "o4-mini" => "settings.modelO4Mini",
        "claude-sonnet-4-5" => "settings.modelClaudeSonnet",
        "claude-opus-4-1" => "settings.modelClaudeOpus",
        "claude-haiku-4-5" => "settings.modelClaudeHaiku",
        _ => return id.to_string(),
    };
    return i18n::t(language.get().as_str(), key);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_and_wire_follow_protocol() {
        assert_eq!(
            resolved_endpoint("https://api.openai.com/", "openai"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            resolved_endpoint("https://gateway.example/llm", "anthropic"),
            "https://gateway.example/llm/v1/messages"
        );
        assert_eq!(resolved_endpoint("", "openai"), "");
        assert_eq!(wire_format("openai"), "POST /v1/chat/completions");
        assert_eq!(wire_format("anthropic"), "POST /v1/messages");
    }
}
