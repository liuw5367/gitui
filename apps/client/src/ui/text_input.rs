//! 文本输入：`nsh_core` 的原生输入框（NSTextField / NSTextView），这里统一外观。
//!
//! 全部输入框走同一套外观：高 28、左右内距 8、圆角 6、1px 描边；聚焦时描边换成
//! 强调色并显示主题 `focus` token 的 2pt outline（不改布局），错误态换成危险色，并在
//! 调用方补一行 11pt 的错误文案（不只靠颜色表达）。
//!
//! "容器 + 内部输入框"的字段（路径、URL、搜索）用 [`field_frame`]：焦点落在内部
//! 输入框上，所以外框必须用 `focus_within`，写在容器上的 `focus_visible` 不会触发。

use crate::design::{self, metrics};
use crate::ui;
use nsh_core::prelude::*;

/// 单行输入框，值受控于 `value`。
pub(crate) fn text_input(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    invalid: bool,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    return style_input(
        nsh_base::input()
            .value(value)
            .placeholder(placeholder)
            .on_change(move |text: &str| on_change(text)),
        invalid,
        Some(ui::CONTROL),
        ui::BODY,
        None,
    );
}

pub(crate) fn text_input_sized(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    invalid: bool,
    height: f32,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    return style_input(
        nsh_base::input()
            .value(value)
            .placeholder(placeholder)
            .on_change(move |text: &str| on_change(text)),
        invalid,
        Some(height),
        ui::BODY,
        None,
    );
}

/// 命令面板搜索框：融入 50pt 顶栏，不绘制独立字段底色或边框。
pub(crate) fn palette_search_input(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    on_change: impl Fn(&str) + 'static,
    auto_focus: bool,
) -> impl IntoElement {
    let placeholder: std::rc::Rc<dyn Fn() -> String> = std::rc::Rc::new(placeholder);
    let label = placeholder.clone();
    return nsh_base::input()
        .value(value)
        .placeholder(move || placeholder())
        .on_change(move |text: &str| on_change(text))
        .auto_focus(auto_focus)
        .w_full()
        .h_full()
        .min_h_0()
        .px(px(0.0))
        .py(px(0.0))
        .border_none()
        .bg(AdaptiveColor::TRANSPARENT)
        .text_size(px(ui::BODY))
        .text_color(move || design::text())
        .label(move || SharedString::from(label()));
}

/// 多行输入框（提交说明、附注、冲突结果）。
pub(crate) fn text_area(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    height: Option<f32>,
    invalid: bool,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    return style_input(
        nsh_base::input()
            .multiline(true)
            .value(value)
            .placeholder(placeholder)
            .on_change(move |text: &str| on_change(text)),
        invalid,
        height,
        ui::BODY,
        None,
    );
}

/// Syntax-highlighted, controlled code editor used by the merge result pane.
pub(crate) fn code_editor(
    path: String,
    get: impl Fn() -> String + 'static,
    placeholder: impl Into<String>,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    return nsh_base::editor()
        .language(path)
        .value(move || SharedString::from(get()))
        .placeholder(placeholder.into())
        .line_numbers(true)
        .soft_wrap(false)
        .font_size(px(ui::CODE))
        .line_height(px(ui::CODE_LINE))
        .on_change(move |text: &SharedString| on_change(text.as_ref()))
        .size_full();
}

/// Syntax-highlighted read-only code pane for merge base/ours/theirs.
pub(crate) fn code_view(
    path: String,
    value: String,
    placeholder: impl Into<String>,
) -> impl IntoElement {
    return nsh_base::editor()
        .language(path)
        .default_value(value)
        .placeholder(placeholder.into())
        .read_only(true)
        .line_numbers(true)
        .soft_wrap(false)
        .font_size(px(ui::CODE))
        .line_height(px(ui::CODE_LINE))
        .size_full();
}

/// 值由结构体信号承载时的输入框（`get`/`set` 各自闭包）。
pub(crate) fn text_input_bound(
    get: impl Fn() -> String + 'static,
    placeholder: impl Fn() -> String + 'static,
    invalid: bool,
    set: impl Fn(String) + 'static,
) -> impl IntoElement {
    return style_input(
        nsh_base::input()
            .value(move || get())
            .placeholder(placeholder)
            .on_change(move |text: &str| set(text.to_string())),
        invalid,
        Some(ui::CONTROL),
        ui::BODY,
        None,
    );
}

/// 值由结构体信号承载、失焦或回车提交的输入框。
///
/// 设置页的 Name / Email 用它：输入过程只改本地信号，`on_blur` / `on_submit`
/// 才写入 git config，避免每按一个键都起一次 `git config`。
pub(crate) fn text_input_bound_commit(
    get: impl Fn() -> String + 'static,
    placeholder: impl Fn() -> String + 'static,
    invalid: bool,
    disabled: bool,
    set: impl Fn(String) + 'static,
    commit: impl Fn() + 'static,
) -> impl IntoElement {
    let commit = std::rc::Rc::new(commit);
    let blur = commit.clone();
    let submit = commit.clone();
    let input = style_input(
        nsh_base::input()
            .readonly(disabled)
            .value(move || get())
            .placeholder(placeholder)
            .on_change(move |text: &str| set(text.to_string()))
            .on_blur(move || blur())
            .on_submit(move |_: &str| submit()),
        invalid,
        Some(ui::CONTROL),
        ui::BODY,
        None,
    );
    let mut wrapper = view().w_full();
    if disabled {
        wrapper = wrapper.opacity(0.45);
    }
    return wrapper.child(input);
}

/// 密码输入：与普通字段同高同描边，内容以圆点遮罩。
pub(crate) fn masked_input(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    on_change: impl Fn(String) + 'static,
) -> impl IntoElement {
    return style_input(
        nsh_base::input()
            .masked(true)
            .value(value)
            .placeholder(placeholder)
            .on_change(move |text: &str| on_change(text.to_string())),
        false,
        Some(ui::CONTROL),
        ui::BODY,
        None,
    );
}

/// 路径输入：输入框 + 右侧键帽，整体一个描边容器。
///
/// 用于 Open 的仓库路径、Clone 的目标目录与 Init 的目录；⌘V 由系统输入框处理，
/// 键帽只作提示。
pub(crate) fn path_input(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    shortcut: Option<&'static str>,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    return path_input_inner(value, placeholder, shortcut, on_change, None);
}

/// 与 [`path_input`] 相同，回车触发 `on_submit`（欢迎页 Open 用它直接打开仓库）。
pub(crate) fn path_input_submit(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    shortcut: Option<&'static str>,
    on_change: impl Fn(&str) + 'static,
    on_submit: impl Fn() + 'static,
) -> impl IntoElement {
    return path_input_inner(
        value,
        placeholder,
        shortcut,
        on_change,
        Some(std::rc::Rc::new(on_submit)),
    );
}

fn path_input_inner(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    shortcut: Option<&'static str>,
    on_change: impl Fn(&str) + 'static,
    on_submit: Option<std::rc::Rc<dyn Fn()>>,
) -> impl IntoElement {
    let mut input = nsh_base::input()
        .value(value)
        .placeholder(placeholder)
        .on_change(move |text: &str| on_change(text))
        .text_size(px(ui::BODY))
        .text_color(move || design::text())
        .flex_1()
        .min_w_0();
    if let Some(submit) = on_submit {
        input = input.on_submit(move |_: &str| submit());
    }
    let mut children: Vec<AnyElement> = vec![input.into_element()];
    if let Some(combo) = shortcut {
        children.push(ui::badge::kbd(combo).into_element());
    }
    return field_frame(children);
}

/// URL 输入：左侧地球图标，右侧无键帽；克隆地址用它。
pub(crate) fn url_input(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    return field_frame(vec![
        ui::icon::icon_view("globe", ui::icon::IconSize::Inline, design::text3).into_element(),
        nsh_base::input()
            .value(value)
            .placeholder(placeholder)
            .on_change(move |text: &str| on_change(text))
            .text_size(px(ui::BODY))
            .text_color(move || design::text())
            .flex_1()
            .min_w_0()
            .into_element(),
    ]);
}

/// 搜索框：左侧放大镜、右侧可选键帽，整体一个描边容器。
///
/// 输入框自身不画边框，避免容器与输入框两道描边；聚焦时由外框换描边色并显示主题 outline。
pub(crate) fn search_field(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    shortcut: Option<&'static str>,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    return search_field_inner(value, placeholder, shortcut, on_change, None);
}

/// 与 [`search_field`] 相同，并把输入框挂到 `handle` 上，供快捷键聚焦。
pub(crate) fn search_field_ref(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    shortcut: Option<&'static str>,
    on_change: impl Fn(&str) + 'static,
    handle: &ElementRef,
) -> impl IntoElement {
    return search_field_inner(value, placeholder, shortcut, on_change, Some(*handle));
}

fn search_field_inner(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    shortcut: Option<&'static str>,
    on_change: impl Fn(&str) + 'static,
    handle: Option<ElementRef>,
) -> impl IntoElement {
    let mut input = nsh_base::input()
        .value(value)
        .placeholder(placeholder)
        .on_change(move |text: &str| on_change(text))
        .text_size(px(ui::BODY))
        .text_color(move || design::text())
        .flex_1()
        .min_w_0();
    if let Some(handle) = handle {
        input = input.element_ref(&handle);
    }
    let mut children: Vec<AnyElement> = vec![
        ui::icon::icon_view("search", ui::icon::IconSize::Inline, design::text3).into_element(),
        input.into_element(),
    ];
    if let Some(combo) = shortcut {
        children.push(ui::badge::kbd(combo).into_element());
    }
    return field_frame(children);
}

/// 输入框外框：所有"容器 + 内部输入框"的字段共用同一套高度、内距、圆角与描边。
///
/// 焦点落在内部输入框上，所以外框用 `focus_within` 变强调色并显示主题 outline；
/// 写在容器上的 `focus_visible` 不会触发。
fn field_frame(children: Vec<AnyElement>) -> impl IntoElement {
    let mut frame = view()
        .w_full()
        .h(px(ui::CONTROL))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_SM))
        .px(px(ui::GAP))
        .rounded(px(metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(move || design::field_border())
        .bg(move || design::field())
        .focus_within(move |style| {
            style
                .border_color(move || design::accent())
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        });
    for child in children {
        frame = frame.child(child);
    }
    return frame;
}

/// 共用外观：描边、底色与文字；聚焦换描边色并显示主题 outline，错误换危险色。
fn style_input(
    input: nsh_core::element::Input,
    invalid: bool,
    height: Option<f32>,
    text_size: f32,
    line_height: Option<f32>,
) -> impl IntoElement {
    let mut input = input.w_full();
    if let Some(height) = height {
        input = input.h(px(height)).min_h(px(height));
    } else {
        input = input.flex_1().min_h(px(0.0));
    }
    let input = input
        .px(px(ui::GAP))
        .py(px(ui::GAP_XS))
        .rounded(px(metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(move || {
            if invalid {
                design::danger()
            } else {
                design::field_border()
            }
        })
        .bg(move || design::field())
        .text_size(px(text_size))
        .text_color(move || design::text());
    if let Some(line_height) = line_height {
        return input.line_height(px(line_height)).focus(move |style| {
            style
                .border_color(move || design::accent())
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        });
    }
    return input.focus(move |style| {
        style
            .border_color(move || design::accent())
            .outline(px(ui::FOCUS_WIDTH))
            .outline_color(move || ui::focus_color())
    });
}
