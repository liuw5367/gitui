mod diff;
mod header;
mod history;
mod statusbar;
mod tabs;

pub(crate) use diff::diff_view;
pub(crate) use header::{header, HeaderActions};
pub(crate) use history::history;
pub(crate) use statusbar::statusbar;
pub(crate) use tabs::tabs;
