//! Git 分区（设计板 24）：可执行文件 + Repository discovery 两张扫描卡。

use super::commands::SettingsCommands;
use crate::design;
use crate::i18n;
use crate::ui;
use nsh_core::prelude::*;
use std::sync::Arc;

pub(super) fn git_page(cmd: SettingsCommands) -> AnyElement {
    let language = cmd.language;
    let settings = cmd.state.settings_state.get_untracked();
    let use_system_git = settings.use_system_git;
    let version = cmd.state.git_version;
    let exec_path = cmd.state.git_exec_path;
    let git_path = cmd.state.git_path;
    let scan_folders = cmd.state.scan_folders;
    let scan_counts = cmd.state.scan_counts;
    let scan_found = cmd.state.scan_found;
    let scan_selected = cmd.state.scan_selected;
    let scan_last_at = cmd.state.scan_last_at;

    let field = ui::text_input::text_input_bound_commit(
        move || {
            if use_system_git {
                exec_path.get()
            } else {
                git_path.get()
            }
        },
        move || i18n::t(language.get().as_str(), "settings.gitPathPlaceholder"),
        false,
        use_system_git,
        {
            let git_path = git_path;
            move |text: String| git_path.set(text)
        },
        {
            let cmd = cmd.clone();
            let git_path = git_path;
            move || cmd.set_git_path(git_path.get_untracked())
        },
    );
    let choose = Arc::new({
        let cmd = cmd.clone();
        move || cmd.choose_git()
    });
    let detect = Arc::new({
        let cmd = cmd.clone();
        move || cmd.detect_git()
    });
    let executable_control = view()
        .flex_col()
        .items_end()
        .gap(px(4.0))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(view().w(px(240.0)).flex_shrink_0().child(field))
                .child(ui::button::button(
                    move || i18n::t(language.get().as_str(), "settings.choose"),
                    ui::button::ButtonVariant::Secondary,
                    ui::button::ButtonSize::Small,
                    use_system_git,
                    {
                        let choose = choose.clone();
                        move || choose()
                    },
                ))
                .child(ui::button::button(
                    move || i18n::t(language.get().as_str(), "settings.detect"),
                    ui::button::ButtonVariant::Secondary,
                    ui::button::ButtonSize::Small,
                    false,
                    {
                        let detect = detect.clone();
                        move || detect()
                    },
                )),
        )
        .child(
            text(move || {
                let raw = version.get();
                if raw.is_empty() {
                    i18n::t(language.get().as_str(), "settings.notDetected")
                } else {
                    i18n::with_arguments(
                        language.get().as_str(),
                        "settings.detected",
                        &[("version", version_label(raw.as_str()))],
                    )
                }
            })
            .text_size(px(ui::SECONDARY))
            .line_height(px(ui::SECONDARY_LINE))
            .text_color(move || design::text3()),
        )
        .into_element();
    let system_git = ui::check::check_control(
        move || i18n::t(language.get().as_str(), "settings.useSystemGit"),
        use_system_git,
        false,
        {
            let cmd = cmd.clone();
            move |next: bool| cmd.set_use_system_git(next)
        },
    );
    let executable = view()
        .w_full()
        .flex_col()
        .child(setting_row(
            language,
            "settings.gitExecutable",
            Some("settings.gitExecutableHint"),
            executable_control,
            true,
        ))
        .child(setting_row(
            language,
            "settings.useSystemGit",
            Some("settings.useSystemGitHint"),
            system_git.into_element(),
            false,
        ))
        .into_element();

    return view()
        .w_full()
        .flex_col()
        .child(ui::setting_row::settings_section(
            true,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.executable")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.executableHint")
            })),
            executable,
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.discovery")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.discoveryHint")
            })),
            discovery(
                language,
                cmd.clone(),
                scan_folders,
                scan_counts,
                scan_found,
                scan_selected,
                scan_last_at,
            ),
        ))
        .into_element();
}

#[allow(clippy::too_many_arguments)]
fn discovery(
    language: Signal<SharedString>,
    cmd: SettingsCommands,
    folders: Signal<Vec<std::path::PathBuf>>,
    counts: Signal<Vec<usize>>,
    found: Signal<Vec<String>>,
    selected: Signal<Vec<String>>,
    last_at: Signal<Option<i64>>,
) -> AnyElement {
    let folder_list = folders.get_untracked();
    let counts_now = counts.get_untracked();
    let found_now = found.get_untracked();
    let selected_now = selected.get_untracked();
    let last = last_at.get_untracked();
    let folder_count = folder_list.len();
    let mut folder_rows = view().w_full().flex_col();
    for (index, folder) in folder_list.iter().enumerate() {
        let path_text = folder.display().to_string();
        let path_remove = path_text.clone();
        let count = counts_now.get(index).copied().unwrap_or(0);
        folder_rows = folder_rows.child(
            view()
                .w_full()
                .h(px(28.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(ui::icon::icon_view(
                    "folder",
                    ui::icon::IconSize::Inline,
                    design::text3,
                ))
                .child(
                    text(move || display_path(path_text.as_str()))
                        .flex_1()
                        .min_w_0()
                        .text_size(px(ui::BODY))
                        .line_height(px(ui::BODY_LINE))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Head),
                )
                .child(
                    text(move || {
                        i18n::with_arguments(
                            language.get().as_str(),
                            "settings.folderRepos",
                            &[("count", count.to_string())],
                        )
                    })
                    .text_size(px(ui::SECONDARY))
                    .text_color(move || design::text3()),
                )
                .child(ui::button::icon_button(
                    "x",
                    move || i18n::t(language.get().as_str(), "settings.removeFolder"),
                    ui::button::ButtonSize::Small,
                    false,
                    {
                        let cmd = cmd.clone();
                        move || cmd.remove_scan_folder(path_remove.clone())
                    },
                )),
        );
    }
    let scan_cmd = cmd.clone();
    let add_folder = cmd.clone();
    let folders_card = ui::feedback::card(
        view()
            .w_full()
            .p(px(12.0))
            .flex_col()
            .gap(px(8.0))
            .child(
                view()
                    .w_full()
                    .flex_row()
                    .items_center()
                    .child(
                        text(move || i18n::t(language.get().as_str(), "settings.scanFolders"))
                            .flex_1()
                            .text_size(px(ui::BODY))
                            .line_height(px(ui::BODY_LINE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(move || design::text()),
                    )
                    .child(ui::badge::count(move || folder_count.to_string())),
            )
            .child(if folder_count == 0 {
                text(move || i18n::t(language.get().as_str(), "settings.scanFoldersEmpty"))
                    .text_size(px(ui::SECONDARY))
                    .line_height(px(ui::SECONDARY_LINE))
                    .text_color(move || design::text3())
                    .into_element()
            } else {
                folder_rows.into_element()
            })
            .child(
                view()
                    .w_full()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .child(ui::button::button(
                        move || i18n::t(language.get().as_str(), "settings.addFolder"),
                        ui::button::ButtonVariant::Secondary,
                        ui::button::ButtonSize::Small,
                        false,
                        {
                            let add_folder = add_folder.clone();
                            move || add_folder.add_scan_folder()
                        },
                    ))
                    .child(ui::button::button(
                        move || i18n::t(language.get().as_str(), "settings.scanNow"),
                        ui::button::ButtonVariant::Secondary,
                        ui::button::ButtonSize::Small,
                        folder_count == 0,
                        {
                            let scan_cmd = scan_cmd.clone();
                            move || scan_cmd.scan_now()
                        },
                    ))
                    .child(view().flex_1())
                    .child(
                        text(move || match last {
                            Some(at) => i18n::with_arguments(
                                language.get().as_str(),
                                "settings.lastScan",
                                &[(
                                    "value",
                                    crate::dates::format(
                                        language.get().as_str(),
                                        at,
                                        store::now_unix(),
                                    ),
                                )],
                            ),
                            None => i18n::t(language.get().as_str(), "settings.neverScanned"),
                        })
                        .text_size(px(ui::SECONDARY))
                        .line_height(px(ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                    ),
            ),
    );
    let found_count = found_now.len();
    let selected_count = selected_now.len();
    let found_empty = found_now.is_empty();
    let all_selected = !found_empty && selected_count == found_count;
    let mut result_rows = view().w_full().flex_col();
    for path in &found_now {
        let selected_here = selected_now.contains(path);
        let path_name = path.clone();
        let display_check = display_path(path.as_str());
        let display_text = display_check.clone();
        let toggle = cmd.clone();
        let toggle_path = path.clone();
        result_rows = result_rows.child(
            view()
                .w_full()
                .h(px(28.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(ui::check::check_control(
                    move || display_check.clone(),
                    selected_here,
                    false,
                    move |_next: bool| toggle.toggle_found(toggle_path.clone()),
                ))
                .child(
                    text(move || {
                        std::path::Path::new(path_name.as_str())
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(path_name.as_str())
                            .to_string()
                    })
                    .text_size(px(ui::BODY))
                    .line_height(px(ui::BODY_LINE))
                    .text_color(move || design::text()),
                )
                .child(
                    text(move || display_text.clone())
                        .flex_1()
                        .min_w_0()
                        .text_size(px(ui::SECONDARY))
                        .line_height(px(ui::SECONDARY_LINE))
                        .font_family(ui::MONO)
                        .text_color(move || design::text3())
                        .text_overflow(nsh_core::EllipsizeMode::Head),
                ),
        );
    }
    let select_cmd = cmd.clone();
    let results_card = ui::feedback::card(
        view()
            .w_full()
            .p(px(12.0))
            .flex_col()
            .gap(px(8.0))
            .child(
                view()
                    .w_full()
                    .flex_row()
                    .items_center()
                    .child(
                        text(move || {
                            i18n::with_arguments(
                                language.get().as_str(),
                                "settings.scanFound",
                                &[("count", found_count.to_string())],
                            )
                        })
                        .flex_1()
                        .text_size(px(ui::BODY))
                        .line_height(px(ui::BODY_LINE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || design::text()),
                    )
                    .child(
                        text(move || {
                            i18n::with_arguments(
                                language.get().as_str(),
                                "settings.searchableCount",
                                &[("count", selected_count.to_string())],
                            )
                        })
                        .text_size(px(ui::SECONDARY))
                        .text_color(move || design::text3()),
                    ),
            )
            .child(if found_empty {
                text(move || i18n::t(language.get().as_str(), "settings.scanEmpty"))
                    .text_size(px(ui::SECONDARY))
                    .line_height(px(ui::SECONDARY_LINE))
                    .text_color(move || design::text3())
                    .into_element()
            } else {
                scroll()
                    .w_full()
                    .max_h(px(220.0))
                    .child(result_rows)
                    .into_element()
            })
            .child(
                view()
                    .w_full()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .child(ui::button::button(
                        move || {
                            i18n::t(
                                language.get().as_str(),
                                if all_selected {
                                    "settings.deselectAll"
                                } else {
                                    "settings.selectAll"
                                },
                            )
                        },
                        ui::button::ButtonVariant::Ghost,
                        ui::button::ButtonSize::Small,
                        found_empty,
                        {
                            let select_cmd = select_cmd.clone();
                            move || select_cmd.select_all_found(!all_selected)
                        },
                    )),
            ),
    );
    return view()
        .w_full()
        .flex_col()
        .gap(px(14.0))
        .child(folders_card)
        .child(results_card)
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

fn version_label(raw: &str) -> String {
    return raw
        .strip_prefix("git version ")
        .map(|rest| format!("Git {rest}"))
        .unwrap_or_else(|| raw.to_string());
}

fn display_path(path: &str) -> String {
    let Some(home) = std::env::var_os("HOME") else {
        return path.to_string();
    };
    let home = home.to_string_lossy();
    if let Some(rest) = path.strip_prefix(home.as_ref()) {
        return format!("~{rest}");
    }
    return path.to_string();
}
