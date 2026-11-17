//! 模态对话框：NSH Modal 管理呈现与关闭语义，UI 层统一绘制遮罩和面板。

use crate::design::{self, metrics};
use crate::ui;
use nsh_base::AlertDialogResult;
use nsh_core::prelude::*;

/// 使用 NSH AlertDialog 显示一组动作。页面与命令通过此入口统一调用基础对话框。
pub(crate) fn alert_with_actions(
    title: String,
    description: String,
    actions: Vec<(&'static str, String)>,
    destructive_action: Option<&'static str>,
    cancel_label: String,
) -> impl Future<Output = Result<AlertDialogResult, String>> + 'static {
    let mut dialog = nsh_base::alert_dialog()
        .title(title)
        .description(description);
    for (id, label) in actions {
        dialog = if destructive_action == Some(id) {
            dialog.destructive(id, label)
        } else {
            dialog.action(id, label)
        };
    }
    return dialog.cancel("cancel", cancel_label).show();
}

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
    return alert_with_actions(
        title,
        description,
        vec![("confirm", action_label)],
        Some("confirm"),
        cancel_label,
    );
}

/// 一个只有「知道了」的提示框。
pub(crate) fn notice(
    title: String,
    description: String,
    cancel_label: String,
) -> impl Future<Output = Result<AlertDialogResult, String>> + 'static {
    return alert_with_actions(title, description, Vec::new(), None, cancel_label);
}

/// 由 NSH Modal 承载应用绘制的遮罩与面板。
///
/// `content` 每次打开时重建，使打开状态跟随 NSH 的条件区域；Esc 与遮罩点击
/// 都回调到调用方，由原有状态源关闭这一层。
pub(crate) fn modal(
    open: impl Fn() -> bool + 'static,
    title: impl Fn() -> String + 'static,
    z_index: i32,
    content: impl Fn() -> AnyElement + 'static,
    on_dismiss: impl Fn() + 'static,
) -> impl IntoElement {
    let title = std::rc::Rc::new(title);
    let content = std::rc::Rc::new(content);
    let on_dismiss = std::rc::Rc::new(on_dismiss);
    return when(open, move || {
        let title = title.clone();
        let content = content.clone();
        let on_dismiss = on_dismiss.clone();
        return modal_open(move || title(), z_index, content(), move || on_dismiss());
    });
}

/// 用于调用方已按状态条件渲染的模态框。
pub(crate) fn modal_open(
    title: impl Fn() -> String + 'static,
    z_index: i32,
    content: impl IntoElement,
    on_dismiss: impl Fn() + 'static,
) -> AnyElement {
    let on_dismiss = std::rc::Rc::new(on_dismiss);
    let cancel = on_dismiss.clone();
    let backdrop = on_dismiss.clone();
    return modal_element(
        title,
        z_index,
        content,
        move || cancel(),
        move || backdrop(),
    )
    .into_element();
}

fn modal_element(
    title: impl Fn() -> String + 'static,
    z_index: i32,
    content: impl IntoElement,
    on_cancel: impl Fn() + 'static,
    on_backdrop: impl Fn() + 'static,
) -> impl IntoElement {
    let title = std::rc::Rc::new(title);
    return nsh_base::dialog()
        .close_on_escape(true)
        // 遮罩由应用绘制，自己的点击处理负责关闭。
        .close_on_backdrop_press(false)
        .label(move || SharedString::from(title()))
        .on_cancel(move || on_cancel())
        .child(
            view()
                .w_full()
                .h_full()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .z_index(z_index)
                .items_center()
                .justify_center()
                .bg(move || crate::design::token("overlay"))
                .on_press(move || on_backdrop())
                .child(content),
        );
}

/// 让内容区截获按压、不再上浮到遮罩。
///
/// 模态面板需要它：不加的话点正文会穿透到遮罩，把对话框关掉。
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
    let title = std::rc::Rc::new(title);
    let panel_title = title.clone();
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
                    text(move || panel_title())
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
    return modal_open(move || title(), 120, panel, on_dismiss);
}
