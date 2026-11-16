//! 屏幕：剩余尚未切片的页面（blame、表单）与工作区版式。
//!
//! 欢迎页、设置页与命令面板已迁到 `features/{welcome,settings,search}/view.rs`；
//! 欢迎页是**独立窗口**（`id = "welcome"`），工作区与设置在同一个窗口的 `main` 里。

mod blame_view;
mod form_dialog;
mod workspace;

pub(crate) use blame_view::blame_view;
pub(crate) use form_dialog::{FormKind, FormState, form_dialog};
pub(crate) use workspace::workspace;
