//! 组件画廊：把 `ui` 层的每个组件、变体与状态并排显示，供实现与验收对照。
//!
//! 这是设计板 22（组件规格页）在实现侧的对等物：22 板描述控件应该长什么样，
//! 这里显示控件实际长什么样。入口是调试用的 `⌘⇧D`，P6 会把入口并入命令面板。

use crate::design;
use crate::ui;
use crate::ui::badge::Tone;
use crate::ui::button::{
    ButtonSize, ButtonVariant, button, button_hint, icon_button, icon_button_tip,
};
use crate::ui::feedback::BannerTone;
use nsh_core::desktop::WindowPosition;
use nsh_core::prelude::*;

/// 组件画廊是独立调试窗口，不参与主窗口的命中测试。
pub(crate) fn gallery(open: Signal<bool>) -> impl IntoElement {
    let input_value = signal(String::new());
    let area_value = signal(String::new());
    let checked = signal(true);
    let switched = signal(false);
    let segment = signal(SharedString::from("history"));
    let select_value = signal(SharedString::from("history"));
    let dialog_open = signal(false);
    return nsh_core::element::desktop::window()
        .id("ui-gallery")
        .title("UI Gallery")
        .default_size(size(Points(1180.0), Points(760.0)))
        .default_position(WindowPosition::Center)
        .open(open)
        .on_open_change(move |change| {
            if !change.open {
                open.set(false);
            }
        })
        .content(move || {
            body(
                open,
                input_value,
                area_value,
                checked,
                switched,
                segment,
                select_value,
                dialog_open,
            )
        });
}

fn body(
    open: Signal<bool>,
    input_value: Signal<String>,
    area_value: Signal<String>,
    checked: Signal<bool>,
    switched: Signal<bool>,
    segment: Signal<SharedString>,
    select_value: Signal<SharedString>,
    dialog_open: Signal<bool>,
) -> impl IntoElement {
    let close = move || open.set(false);
    return scroll().w_full().h_full().child(
        view()
            .w_full()
            .p(px(ui::GAP_XL))
            .flex_col()
            .gap(px(ui::GAP_XL))
            .child(
                view()
                    .w_full()
                    .flex_row()
                    .items_center()
                    .gap(px(ui::GAP))
                    .child(
                        text("ui gallery")
                            .flex_1()
                            .text_size(px(ui::BODY))
                            .line_height(px(ui::BODY_LINE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(move || design::text()),
                    )
                    .child(ui::badge::kbd("⌘⇧D"))
                    .child(button(
                        move || "Close".to_string(),
                        ButtonVariant::Secondary,
                        ButtonSize::Regular,
                        false,
                        close,
                    )),
            )
            .child(section(
                "Buttons",
                view()
                    .w_full()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(px(ui::GAP))
                    .child(button(
                        move || "Primary".to_string(),
                        ButtonVariant::Primary,
                        ButtonSize::Regular,
                        false,
                        || {},
                    ))
                    .child(button(
                        move || "Secondary".to_string(),
                        ButtonVariant::Secondary,
                        ButtonSize::Regular,
                        false,
                        || {},
                    ))
                    .child(button(
                        move || "Ghost".to_string(),
                        ButtonVariant::Ghost,
                        ButtonSize::Regular,
                        false,
                        || {},
                    ))
                    .child(button(
                        move || "Danger".to_string(),
                        ButtonVariant::Danger,
                        ButtonSize::Regular,
                        false,
                        || {},
                    ))
                    .child(button(
                        move || "Disabled".to_string(),
                        ButtonVariant::Primary,
                        ButtonSize::Regular,
                        true,
                        || {},
                    ))
                    .child(button_hint(
                        move || "Open Repository".to_string(),
                        || "⌘O".to_string(),
                        ButtonVariant::Primary,
                        ButtonSize::Regular,
                        false,
                        || {},
                    ))
                    .child(button(
                        move || "Small".to_string(),
                        ButtonVariant::Secondary,
                        ButtonSize::Small,
                        false,
                        || {},
                    ))
                    .child(button(
                        move || "Large".to_string(),
                        ButtonVariant::Primary,
                        ButtonSize::Large,
                        false,
                        || {},
                    ))
                    .child(icon_button(
                        "plus",
                        move || "Icon".to_string(),
                        ButtonSize::Regular,
                        false,
                        || {},
                    ))
                    .child(icon_button(
                        "copy",
                        move || "Disabled icon".to_string(),
                        ButtonSize::Small,
                        true,
                        || {},
                    ))
                    .child(icon_button_tip(
                        "gear",
                        move || "Settings".to_string(),
                        ButtonSize::Small,
                        false,
                        move || "Hover shows this hint".to_string(),
                        || {},
                    )),
            ))
            .child(section(
                "Fields",
                view()
                    .w_full()
                    .flex_col()
                    .gap(px(ui::GAP_LG))
                    .child(ui::field::field(
                        move || "Text field focus outline".to_string(),
                        ui::text_input::text_input(
                            input_value,
                            move || "Type here".to_string(),
                            false,
                            |_text| {},
                        ),
                        Some(Box::new(move || {
                            "Tab here to inspect the 2pt theme focus ring.".to_string()
                        })),
                        None,
                    ))
                    .child(ui::field::field(
                        move || "Invalid field".to_string(),
                        ui::text_input::text_input(
                            input_value,
                            move || "Required".to_string(),
                            true,
                            |_text| {},
                        ),
                        None,
                        Some(Box::new(move || "This value is required.".to_string())),
                    ))
                    .child(ui::field::field(
                        move || "Path with keycap".to_string(),
                        ui::text_input::path_input(
                            input_value,
                            move || "~/Developer/repo".to_string(),
                            Some("⌘V"),
                            move |text| input_value.set(text.to_string()),
                        ),
                        Some(Box::new(move || {
                            "Paste a path or choose a folder.".to_string()
                        })),
                        None,
                    ))
                    .child(ui::field::field(
                        move || "URL field".to_string(),
                        ui::text_input::url_input(
                            input_value,
                            move || "https://github.com/owner/repo.git".to_string(),
                            move |text| input_value.set(text.to_string()),
                        ),
                        None,
                        None,
                    ))
                    .child(ui::field::field(
                        move || "Search focus outline".to_string(),
                        ui::text_input::search_field(
                            input_value,
                            move || "Search repositories".to_string(),
                            Some("⌘F"),
                            |_text| {},
                        ),
                        Some(Box::new(move || {
                            "The outer frame follows the focused search input.".to_string()
                        })),
                        None,
                    ))
                    .child(ui::field::field(
                        move || "Multiline focus outline".to_string(),
                        ui::text_input::text_area(
                            area_value,
                            move || "Description".to_string(),
                            Some(88.0),
                            false,
                            |_text| {},
                        ),
                        Some(Box::new(move || {
                            "Focus the editor to inspect the theme ring.".to_string()
                        })),
                        None,
                    )),
            ))
            .child(section(
                "Rows",
                view()
                    .w_full()
                    .flex_col()
                    .gap(px(ui::GAP))
                    .child(ui::row::row_sized(
                        move || "gitui".to_string(),
                        Some((Box::new(move || "~/Developer/gitui".to_string()), true)),
                        Some(
                            ui::icon::icon_view(
                                "folder",
                                ui::icon::IconSize::Inline,
                                design::text2,
                            )
                            .into_element(),
                        ),
                        vec![ui::badge::branch_chip(move || "main".to_string()).into_element()],
                        false,
                        false,
                        47.0,
                        || {},
                    ))
                    .child(ui::row::row_stacked(
                        move || "atlas-web".to_string(),
                        Some((Box::new(move || "~/Developer/atlas-web".to_string()), true)),
                        Some(
                            ui::icon::icon_view(
                                "folder",
                                ui::icon::IconSize::Inline,
                                design::accent,
                            )
                            .into_element(),
                        ),
                        vec![
                            ui::badge::branch_chip(move || "feature/hero".to_string())
                                .into_element(),
                        ],
                        vec![
                            ui::meta(move || "2m".to_string()).into_element(),
                            icon_button(
                                "x",
                                move || "Remove".to_string(),
                                ButtonSize::Small,
                                false,
                                || {},
                            )
                            .into_element(),
                        ],
                        true,
                        false,
                        47.0,
                        || {},
                    ))
                    .child(ui::row::row_stacked(
                        move || "A repository with a very long name".to_string(),
                        Some((
                            Box::new(move || "~/Developer/a-very-long-path".to_string()),
                            true,
                        )),
                        Some(
                            ui::icon::icon_view(
                                "folder",
                                ui::icon::IconSize::Inline,
                                design::text2,
                            )
                            .into_element(),
                        ),
                        vec![ui::badge::branch_chip(move || "release".to_string()).into_element()],
                        vec![ui::meta(move || "Nov 6".to_string()).into_element()],
                        false,
                        false,
                        47.0,
                        || {},
                    )),
            ))
            .child(section(
                "Selection",
                view()
                    .w_full()
                    .flex_col()
                    .gap(px(ui::GAP))
                    .child(dynamic(move || {
                        let value = checked.get();
                        return ui::check::check_row_button(
                            move || "Checkbox row".to_string(),
                            value,
                            move |next| checked.set(next),
                        )
                        .into_element();
                    }))
                    .child(ui::check::check_row(
                        move || "Disabled checkbox".to_string(),
                        false,
                        true,
                        |_next| {},
                    ))
                    .child(ui::check::switch_row(
                        move || "Switch row".to_string(),
                        Some(Box::new(move || "With a help line".to_string())),
                        switched.get(),
                        false,
                        move |next| switched.set(next),
                    ))
                    .child(dynamic(move || {
                        let selected = match segment.get().as_ref() {
                            "changes" => "changes",
                            "refs" => "refs",
                            _ => "history",
                        };
                        return ui::segmented::segmented(
                            vec![
                                ui::segmented::Segment {
                                    id: "history",
                                    label: Box::new(move || "History".to_string()),
                                },
                                ui::segmented::Segment {
                                    id: "changes",
                                    label: Box::new(move || "Changes".to_string()),
                                },
                                ui::segmented::Segment {
                                    id: "refs",
                                    label: Box::new(move || "Refs".to_string()),
                                },
                            ],
                            selected,
                            move |id| segment.set(SharedString::from(id)),
                        )
                        .into_element();
                    }))
                    .child(
                        view()
                            .w_full()
                            .flex_row()
                            .items_center()
                            .gap(px(ui::GAP))
                            .child(dynamic(move || {
                                return ui::select::select(
                                    vec![
                                        ui::select::SelectOption::new("history", || {
                                            "History".to_string()
                                        }),
                                        ui::select::SelectOption::new("changes", || {
                                            "Changes".to_string()
                                        }),
                                        ui::select::SelectOption::new("refs", || {
                                            "References".to_string()
                                        }),
                                    ],
                                    select_value.get(),
                                    Some(210.0),
                                    false,
                                    move |id| select_value.set(SharedString::from(id)),
                                )
                                .into_element();
                            }))
                            .child(ui::select::select(
                                vec![ui::select::SelectOption::new("disabled", || {
                                    "Disabled select".to_string()
                                })],
                                "disabled",
                                Some(210.0),
                                true,
                                |_| {},
                            )),
                    ),
            ))
            .child(section(
                "Badges & keys",
                view()
                    .w_full()
                    .flex_row()
                    .items_center()
                    .gap(px(ui::GAP))
                    .child(ui::badge::badge(
                        move || "Neutral".to_string(),
                        Tone::Neutral,
                    ))
                    .child(ui::badge::badge(move || "Accent".to_string(), Tone::Accent))
                    .child(ui::badge::badge(
                        move || "Success".to_string(),
                        Tone::Success,
                    ))
                    .child(ui::badge::badge(
                        move || "Warning".to_string(),
                        Tone::Warning,
                    ))
                    .child(ui::badge::badge(move || "Danger".to_string(), Tone::Danger))
                    .child(ui::badge::kbd("⌘P"))
                    .child(ui::badge::kbd("⇧⌘F"))
                    .child(ui::badge::branch_chip(move || "feature/hero".to_string())),
            ))
            .child(section(
                "Banners",
                view()
                    .w_full()
                    .flex_col()
                    .gap(px(ui::GAP))
                    .child(ui::feedback::banner(
                        BannerTone::Info,
                        move || "Fetched 3 commits from origin.".to_string(),
                        vec![],
                    ))
                    .child(ui::feedback::banner(
                        BannerTone::Warning,
                        move || "Working tree is dirty.".to_string(),
                        vec![],
                    ))
                    .child(ui::feedback::banner(
                        BannerTone::Danger,
                        move || "Merge conflict in 2 files.".to_string(),
                        vec![],
                    ))
                    .child(ui::feedback::progress(Some(0.4)))
                    .child(ui::feedback::progress(None)),
            ))
            .child(section(
                "Sections & empty",
                view()
                    .w_full()
                    .flex_col()
                    .gap(px(ui::GAP))
                    .child(ui::section::pane_header(
                        move || "History".to_string(),
                        Some(Box::new(move || "128".to_string())),
                        vec![ui::badge::kbd("⌘F").into_element()],
                    ))
                    .child(ui::section::section_label(move || "Branches".to_string()))
                    .child(ui::section::section_label_upper(move || {
                        "Today".to_string()
                    }))
                    .child(ui::section::skeleton_row(&[120.0, 64.0, 40.0]))
                    .child(
                        view()
                            .w_full()
                            .h(px(160.0))
                            .rounded(px(8.0))
                            .border_1()
                            .border_color(move || design::line())
                            .child(ui::section::empty(
                                move || "Nothing to show".to_string(),
                                Some(Box::new(move || {
                                    "This repository has no commits yet.".to_string()
                                })),
                                None,
                            )),
                    )
                    .child(ui::section::empty_card(
                        "folder-plus",
                        move || "No repositories yet".to_string(),
                        Some(Box::new(move || {
                            "Open a local repository, or clone and create one.".to_string()
                        })),
                        Some(
                            button(
                                move || "Scan Repositories".to_string(),
                                ButtonVariant::Secondary,
                                ButtonSize::Small,
                                false,
                                || {},
                            )
                            .into_element(),
                        ),
                    )),
            ))
            .child(section(
                "Dialog",
                view()
                    .w_full()
                    .flex_row()
                    .items_center()
                    .gap(px(ui::GAP))
                    .child(button(
                        move || "Open dialog".to_string(),
                        ButtonVariant::Secondary,
                        ButtonSize::Regular,
                        false,
                        move || dialog_open.set(true),
                    )),
            ))
            .child(ui::dialog::dialog(
                dialog_open.get(),
                420.0,
                move || "New branch".to_string(),
                view()
                    .w_full()
                    .flex_col()
                    .gap(px(ui::GAP_LG))
                    .child(ui::field::field(
                        move || "Name".to_string(),
                        ui::text_input::text_input(
                            signal(String::new()),
                            move || "feature/".to_string(),
                            false,
                            |_text| {},
                        ),
                        None,
                        None,
                    )),
                vec![
                    button(
                        move || "Cancel".to_string(),
                        ButtonVariant::Secondary,
                        ButtonSize::Regular,
                        false,
                        move || dialog_open.set(false),
                    )
                    .into_element(),
                    button(
                        move || "Create".to_string(),
                        ButtonVariant::Primary,
                        ButtonSize::Regular,
                        false,
                        move || dialog_open.set(false),
                    )
                    .into_element(),
                ],
                move || dialog_open.set(false),
            )),
    );
}

/// 一组组件：11pt 标题 + 内容。
fn section(title: &'static str, content: impl IntoElement) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(ui::GAP))
        .child(
            text(title)
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || design::text3()),
        )
        .child(ui::feedback::card(content));
}
