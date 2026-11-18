//! 提交历史功能：状态、用例与渲染。

pub(crate) mod blame;
pub(crate) mod commands;
pub(crate) mod graph;
pub(crate) mod inspector;
pub(crate) mod ops;
pub(crate) mod state;
pub(crate) mod view;

pub(crate) use blame::blame_view;
