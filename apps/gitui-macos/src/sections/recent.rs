//! Recent repository chips.

use nsh_base::theme;
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::model::short_name;

pub(crate) fn recent_bar(
    recent: Signal<Vec<SharedString>>,
    on_open: Arc<dyn Fn(String)>,
) -> impl IntoElement {
    view()
        .h(px(28.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .bg(move || theme().background)
        .child(
            text("Recent")
                .text_size(px(9.0))
                .text_color(move || theme().foreground),
        )
        .child(each(
            recent,
            |path: &SharedString| path.clone(),
            move |path, _| {
                let on_open = on_open.clone();
                view()
                    .h(px(20.0))
                    .px_2()
                    .rounded(px(4.0))
                    .bg(move || theme().primary)
                    .on_press(move || on_open(path.get().to_string()))
                    .child(
                        text(move || short_name(&path.get().to_string()))
                            .text_size(px(9.0))
                            .text_color(move || theme().primary_foreground),
                    )
            },
        ))
}
