//! 列表行与导航行：行级按压目标统一在这里，页面不再直接给 `view()` 挂 `.on_press`。
//!
//! 单行 28、双行 44；选中态用强调底与正文色；行内图标、徽章、计数由调用方以
//! `ui` 组件作为前后槽传入。行为（按压、禁用、键盘可达）来自 `nsh_base::button`。

use crate::design;
use crate::ui;
use nsh_core::prelude::*;

const PALETTE_ROW_HEIGHT: f32 = 44.0;
const PALETTE_ROW_GAP: f32 = 9.0;
const PALETTE_ROW_PADDING: f32 = 9.0;
const PALETTE_ROW_RADIUS: f32 = 7.0;

/// 一行列表项。
pub(crate) fn row(
    caption: impl Fn() -> String + 'static,
    subtitle: Option<(Box<dyn Fn() -> String>, bool)>,
    leading: Option<AnyElement>,
    trailing: Vec<AnyElement>,
    selected: bool,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let height = if subtitle.is_some() { 44.0 } else { ui::ROW };
    return row_sized(
        caption, subtitle, leading, trailing, selected, disabled, height, on_press,
    );
}

/// 两行仓库行：左侧图标，右侧第一行「名称 + 行内附属 + 行尾动作」，第二行路径。
///
/// 与 [`row_sized`] 的区别只在行内结构：`row_sized` 把 trailing 放在两行文字的右侧，
/// 这里放在第一行右侧，第二行因此占满整行宽度（欢迎页画板的 `.wrepo`）。
/// 两行都是单行文本；名称从右侧省略，路径从左侧省略（保留仓库目录名这一端）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn row_stacked(
    caption: impl Fn() -> String + 'static,
    subtitle: Option<(Box<dyn Fn() -> String>, bool)>,
    leading: Option<AnyElement>,
    aside: Vec<AnyElement>,
    trailing: Vec<AnyElement>,
    selected: bool,
    disabled: bool,
    height: f32,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let mut element = nsh_base::button()
        .disabled(disabled)
        .opacity(if disabled { 0.45 } else { 1.0 })
        .w_full()
        .h(px(height))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_SM))
        .px(px(ui::GAP))
        .rounded(px(6.0))
        .bg(move || {
            if selected {
                design::selected()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .hover(move |style| {
            if selected {
                style
            } else {
                style.bg(move || design::hover())
            }
        })
        .focus_visible(move |style| {
            style
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        })
        .on_press(move |_: &()| on_press());
    if let Some(leading) = leading {
        element = element.child(leading);
    }
    // 第一行：名称（可省略）、行内附属（分支 chip）、弹性空隙、行尾（时间、删除）。
    let foreground = design::text();
    let mut first = view()
        .w_full()
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_XS))
        .child(
            view().min_w_0().flex_shrink(1.0).child(
                text(move || caption())
                    .text_size(px(ui::BODY))
                    .line_height(px(ui::BODY_LINE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(move || foreground)
                    .number_of_lines(1)
                    .text_overflow(nsh_core::EllipsizeMode::Tail),
            ),
        );
    for item in aside {
        first = first.child(item);
    }
    first = first.child(view().flex_1());
    for item in trailing {
        first = first.child(item);
    }
    // 标签列用 flex_1 而不是 w_full：后者会占满整行，把行尾的动作挤出可视区。
    let mut labels = view().flex_1().min_w_0().flex_col().child(first);
    if let Some((hint, mono)) = subtitle {
        // 副标题只占一行，并且从左侧省略：路径的尾部（仓库目录名）比开头更有用。
        let mut line = text(move || hint())
            .text_size(px(ui::SECONDARY))
            .line_height(px(ui::SECONDARY_LINE))
            .text_color(move || design::text3())
            .number_of_lines(1)
            .text_overflow(nsh_core::EllipsizeMode::Head);
        if mono {
            line = line.font_family(ui::MONO);
        }
        labels = labels.child(line);
    }
    return element.child(labels);
}

/// 同 [`row`]，行高由调用方给。
pub(crate) fn row_sized(
    caption: impl Fn() -> String + 'static,
    subtitle: Option<(Box<dyn Fn() -> String>, bool)>,
    leading: Option<AnyElement>,
    trailing: Vec<AnyElement>,
    selected: bool,
    disabled: bool,
    height: f32,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return row_sized_with_radius(
        caption, subtitle, leading, trailing, selected, disabled, height, 6.0, on_press,
    );
}

/// 行高由调用方给、背景保持平直的单行按钮。
pub(crate) fn row_sized_flat(
    caption: impl Fn() -> String + 'static,
    subtitle: Option<(Box<dyn Fn() -> String>, bool)>,
    leading: Option<AnyElement>,
    trailing: Vec<AnyElement>,
    selected: bool,
    disabled: bool,
    height: f32,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return row_sized_with_radius(
        caption, subtitle, leading, trailing, selected, disabled, height, 0.0, on_press,
    );
}

#[allow(clippy::too_many_arguments)]
fn row_sized_with_radius(
    caption: impl Fn() -> String + 'static,
    subtitle: Option<(Box<dyn Fn() -> String>, bool)>,
    leading: Option<AnyElement>,
    trailing: Vec<AnyElement>,
    selected: bool,
    disabled: bool,
    height: f32,
    radius: f32,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let mut element = nsh_base::button()
        .disabled(disabled)
        .opacity(if disabled { 0.45 } else { 1.0 })
        .w_full()
        .h(px(height))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_SM))
        .px(px(ui::GAP))
        .rounded(px(radius))
        .bg(move || {
            if selected {
                design::selected()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .hover(move |style| {
            if selected {
                style
            } else {
                style.bg(move || design::hover())
            }
        })
        .focus_visible(move |style| {
            style
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        })
        .on_press(move |_: &()| on_press());
    if let Some(leading) = leading {
        element = element.child(leading);
    }
    // 选中行用 sel 底色；文字保持正文色，sel-strong 与底色对比度不足。
    let foreground = design::text();
    let mut labels = view().flex_1().min_w_0().flex_col().child(
        text(move || caption())
            .text_size(px(ui::BODY))
            .line_height(px(ui::BODY_LINE))
            .text_color(move || foreground)
            .number_of_lines(1)
            .text_overflow(nsh_core::EllipsizeMode::Tail),
    );
    if let Some((hint, mono)) = subtitle {
        // 副标题只占一行，并且从左侧省略：路径的尾部（仓库目录名）比开头更有用。
        let mut line = text(move || hint())
            .text_size(px(ui::SECONDARY))
            .line_height(px(ui::SECONDARY_LINE))
            .text_color(move || design::text3())
            .number_of_lines(1)
            .text_overflow(nsh_core::EllipsizeMode::Head);
        if mono {
            line = line.font_family(ui::MONO);
        }
        labels = labels.child(line);
    }
    element = element.child(labels);
    for item in trailing {
        element = element.child(item);
    }
    return element;
}

/// A selectable blame source line. The code column stays monospaced while the author gutter
/// can truncate independently inside the fixed width reserved by the blame board.
pub(crate) fn blame_line(
    short: String,
    author: String,
    line: u32,
    code: String,
    selected: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let mut row = nsh_base::button()
        .w_full()
        .h(px(ui::CODE_LINE))
        .flex_row()
        .items_center()
        .px(px(ui::GAP_SM))
        .bg(move || {
            if selected {
                design::selected()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .hover(move |style| {
            if selected {
                style
            } else {
                style.bg(move || design::hover())
            }
        })
        .focus_visible(move |style| {
            style
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        })
        .on_press(move |_: &()| on_press());

    row = row.child(view().w(px(2.0)).h_full().flex_shrink_0().bg(move || {
        if selected {
            design::accent()
        } else {
            AdaptiveColor::TRANSPARENT
        }
    }));
    row = row.child(
        view()
            .w(px(124.0))
            .h_full()
            .flex_shrink_0()
            .flex_row()
            .items_center()
            .gap(px(ui::GAP_XS))
            .pr(px(ui::GAP_SM))
            .border_r_1()
            .border_color(move || design::line_soft())
            .child(
                text(short)
                    .w(px(48.0))
                    .flex_shrink_0()
                    .font_family(ui::MONO)
                    .text_size(px(ui::META))
                    .line_height(px(ui::CODE_LINE))
                    .text_color(move || design::text2())
                    .text_overflow(nsh_core::EllipsizeMode::Tail),
            )
            .child(
                text(author)
                    .flex_1()
                    .min_w_0()
                    .text_size(px(ui::META))
                    .line_height(px(ui::CODE_LINE))
                    .text_color(move || design::text3())
                    .text_overflow(nsh_core::EllipsizeMode::Tail),
            ),
    );
    row = row.child(
        text(line.to_string())
            .w(px(32.0))
            .flex_shrink_0()
            .pr(px(ui::GAP_SM))
            .font_family(ui::MONO)
            .text_size(px(ui::META))
            .line_height(px(ui::CODE_LINE))
            .text_color(move || design::text3()),
    );
    return row.child(
        text(code)
            .flex_1()
            .min_w_0()
            .font_family(ui::MONO)
            .text_size(px(ui::CODE))
            .line_height(px(ui::CODE_LINE))
            .text_color(move || design::text())
            .text_overflow(nsh_core::EllipsizeMode::Clip),
    );
}

/// 命令面板的双行结果：沿用列表行的语义与焦点行为，使用画板的间距和圆角。
pub(crate) fn palette_row(
    caption: impl Fn() -> String + 'static,
    subtitle: Option<(Box<dyn Fn() -> String>, bool)>,
    leading: Option<AnyElement>,
    trailing: Vec<AnyElement>,
    is_selected: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let is_selected: std::rc::Rc<dyn Fn() -> bool> = std::rc::Rc::new(is_selected);
    let selected_for_background = is_selected.clone();
    let mut element = nsh_base::button()
        .w_full()
        .h(px(PALETTE_ROW_HEIGHT))
        .flex_row()
        .items_center()
        .gap(px(PALETTE_ROW_GAP))
        .px(px(PALETTE_ROW_PADDING))
        .rounded(px(PALETTE_ROW_RADIUS))
        .bg(move || {
            if selected_for_background() {
                design::selected()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .hover({
            let is_selected = is_selected.clone();
            move |style| {
                let is_selected = is_selected.clone();
                style.bg(move || {
                    if is_selected() {
                        design::token("sel-strong")
                    } else {
                        design::hover()
                    }
                })
            }
        })
        .focus_visible(move |style| {
            style
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        })
        .on_press(move |_: &()| on_press());
    if let Some(leading) = leading {
        element = element.child(leading);
    }

    let mut labels = view().flex_1().min_w_0().flex_col().child(
        text(move || caption())
            .text_size(px(ui::BODY))
            .line_height(px(ui::BODY_LINE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(move || design::text())
            .number_of_lines(1)
            .text_overflow(nsh_core::EllipsizeMode::Tail),
    );
    if let Some((hint, mono)) = subtitle {
        let mut line = text(move || hint())
            .text_size(px(ui::SECONDARY))
            .line_height(px(ui::SECONDARY_LINE))
            .text_color(move || design::text3())
            .number_of_lines(1)
            .text_overflow(nsh_core::EllipsizeMode::Head);
        if mono {
            line = line.font_family(ui::MONO);
        }
        labels = labels.child(line);
    }
    element = element.child(labels);
    for item in trailing {
        element = element.child(item);
    }
    return element;
}

/// 可折叠分组标题：左侧状态图标、标题、计数等右侧内容与键盘可达的按压行为。
pub(crate) fn section_header(
    caption: impl Fn() -> String + 'static,
    leading: AnyElement,
    trailing: Vec<AnyElement>,
    height: f32,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let mut element = nsh_base::button()
        .w_full()
        .h(px(height))
        .px(px(6.0))
        .flex_row()
        .items_center()
        .gap(px(5.0))
        .rounded(px(6.0))
        .bg(AdaptiveColor::TRANSPARENT)
        .hover(|style| style.bg(move || design::hover()))
        .focus_visible(move |style| {
            style
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        })
        .on_press(move |_: &()| on_press())
        .child(leading)
        .child(
            text(move || caption())
                .text_size(px(ui::SECONDARY))
                .line_height(px(height))
                .font_weight(FontWeight(600))
                .text_color(move || design::text3()),
        )
        .child(view().flex_1());
    for item in trailing {
        element = element.child(item);
    }
    return element;
}

/// 设置页左树的导航行：28 高、强调底 + 白色文字/图标。
pub(crate) fn nav_row(
    label: impl Fn() -> String + 'static,
    icon: &'static str,
    selected: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let foreground = move || {
        if selected {
            design::token("primary-foreground")
        } else {
            design::text()
        }
    };
    return nsh_base::button()
        .h(px(28.0))
        .w_full()
        .px(px(8.0))
        .flex_row()
        .items_center()
        .gap(px(7.0))
        .rounded(px(6.0))
        .bg(move || {
            if selected {
                design::accent()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .hover(move |style| {
            if selected {
                style
            } else {
                style.bg(move || design::hover())
            }
        })
        .focus_visible(move |style| {
            style
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        })
        .on_press(move |_: &()| on_press())
        .child(
            view()
                .w(px(16.0))
                .items_center()
                .justify_center()
                .child(ui::icon::icon_view(
                    icon,
                    ui::icon::IconSize::Inline,
                    move || {
                        if selected {
                            design::token("primary-foreground")
                        } else {
                            design::text3()
                        }
                    },
                )),
        )
        .child(
            text(move || label())
                .text_size(px(ui::BODY))
                .line_height(px(ui::BODY_LINE))
                .font_weight(if selected {
                    FontWeight::MEDIUM
                } else {
                    FontWeight(400)
                })
                .text_color(foreground),
        );
}
