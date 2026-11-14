mod diff;
mod header;
mod operation;
mod history;
mod recent;
mod statusbar;
mod tabs;

pub(crate) use diff::diff_view;
pub(crate) use header::{header, HeaderActions};
pub(crate) use operation::operation_banner;
pub(crate) use history::history;
pub(crate) use recent::recent_bar;
pub(crate) use statusbar::statusbar;
pub(crate) use tabs::tabs;
