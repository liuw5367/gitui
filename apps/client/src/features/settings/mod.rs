//! 设置页功能：状态、用例与渲染；分区按页拆在 `general` / `git` / `shortcuts` /
//! `about` / `agent` 里（设计板 23–27）。

pub(crate) mod about;
pub(crate) mod agent;
pub(crate) mod commands;
pub(crate) mod general;
pub(crate) mod git;
pub(crate) mod shortcuts;
pub(crate) mod state;
pub(crate) mod view;
