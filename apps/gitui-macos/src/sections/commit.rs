//! Commit composer actions.

use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct CommitActions {
    pub(crate) commit: Arc<dyn Fn()>,
    pub(crate) no_hooks: Arc<dyn Fn()>,
    pub(crate) amend: Arc<dyn Fn()>,
}
