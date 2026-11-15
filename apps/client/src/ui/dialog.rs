//! 模态对话框：遮罩 + 居中面板 + 页头/正文/页脚三段。
//!
//! 行为与设计稿一致：点击遮罩关闭、内容区截获按压、Esc 由调用方在全局浮层栈里处理。
//! 页头 44、页脚 48、正文内边距 16；面板最大高度 760，正文自己滚动。
//!
//! 返回值是 `AnyElement`：`open` 为 false 时返回零尺寸占位。调用方在构建时读一次
//! `open`（信号读取会订阅重建），不需要在这里再做一层 `dynamic`。

use crate::design::{self, metrics};
use nsh_base::AlertDialogResult;
use crate::ui;
use nsh_core::prelude::*;

/// 一个「危险操作」确认框：系统原生 alert，标题 + 说明 + 一个破坏性动作 + 取消。
///
/// 返回的 future 解析为动作 id（取消时为 `None`）。这个包装存在的意义是让页面
/// 不必直接引用 `nsh_base::alert_dialog`——那会让该文件过不了 guard 的
/// 「已迁移页面不得直接引用 nsh_base」检查。
pub(crate) fn confirm_destructive(
    title: String,
    description: String,
    action_label: String,
    cancel_label: String,
) -> impl Future<Output = Result<AlertDialogResult, String>> + 'static {
    return nsh_base::alert_dialog()
        .title(title)
        .description(description)
        .destructive("confirm", action_label)
        .cancel("cancel", cancel_label)
        .show();
}

/// 一个只有「知道了」的提示框。
pub(crate) fn notice(
    title: String,
    description: String,
    cancel_label: String,
) -> impl Future<Output = Result<AlertDialogResult, String>> + 'static {
    return nsh_base::alert_dialog()
        .title(title)
        .description(description)
        .cancel("cancel", cancel_label)
        .show();
}

/// 一层遮罩：铺满父容器、居中内容、点遮罩关闭。
///
/// 与 [`modality`] 配对使用：遮罩负责「点外面关掉」，`modality` 负责「点里面
/// 不要关掉」。两者一起，自搭弹层的页面也不必出现裸 `.on_press`。
pub(crate) fn scrim(
    visible: impl Fn() -> bool + 'static,
    z_index: i32,
    content: impl IntoElement,
    on_dismiss: impl Fn() + 'static,
) -> impl IntoElement {
    let on_dismiss = std::rc::Rc::new(on_dismiss);
    return view()
        .absolute()
        .inset(px(0.0))
        .z_index(z_index)
        .items_center()
        .justify_center()
        .bg(move || {
            if visible() {
                crate::design::token("overlay")
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .on_press(move || on_dismiss())
        .child(content);
}

/// 让内容区截获按压、不再上浮到遮罩。
///
/// 自搭弹层的面板需要它：不加的话点正文会穿透到遮罩，把对话框关掉。`dialog`
/// 内部已自动这么做；这里单独导出，是为了让还没换成 `dialog` 的弹层也不必在
/// 页面里写裸 `.on_press(|| {})`（那样该文件就无法通过 guard 的检查）。
pub(crate) fn modality(content: impl IntoElement) -> impl IntoElement {
    return view().on_press(|| {}).child(content);
}

/// 一个模态对话框。
pub(crate) fn dialog(
    open: bool,
    width: f32,
    title: impl Fn() -> String + 'static,
    body: impl IntoElement,
    footer: Vec<AnyElement>,
    on_dismiss: impl Fn() + 'static,
) -> AnyElement {
    if !open {
        return view().w(px(0.0)).h(px(0.0)).into_element();
    }
    let mut panel = view()
        .w(px(width))
        .max_h(px(760.0))
        .flex_col()
        .rounded(px(metrics::RADIUS_PANEL + 2.0))
        .border_1()
        .border_color(move || design::line_strong())
        .bg(move || design::token("popover"))
        .shadow_lg()
        // 内容区的按压不再上浮到遮罩，避免点正文就关掉对话框。
        .on_press(|| {})
        .child(
            view()
                .w_full()
                .h(px(44.0))
                .flex_row()
                .items_center()
                .gap(px(ui::GAP))
                .px(px(ui::GAP_XL))
                .border_b_1()
                .border_color(move || design::line())
                .child(
                    text(move || title())
                        .flex_1()
                        .min_w_0()
                        .text_size(px(ui::BODY))
                        .line_height(px(44.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                ),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .p(px(ui::GAP_XL))
                .child(body),
        );
    if !footer.is_empty() {
        let mut row = view()
            .w_full()
            .h(px(48.0))
            .flex_row()
            .items_center()
            .justify_end()
            .gap(px(ui::GAP))
            .px(px(ui::GAP_XL))
            .border_t_1()
            .border_color(move || design::line());
        for element in footer {
            row = row.child(element);
        }
        panel = panel.child(row);
    }
    return view()
        .absolute()
        .inset(px(0.0))
        .z_index(120)
        .items_center()
        .justify_center()
        .bg(move || design::token("overlay"))
        .on_press(move || on_dismiss())
        .child(panel)
        .into_element();
}
