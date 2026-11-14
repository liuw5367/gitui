//! Stash bar.

use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct StashActions {
    pub(crate) push: Arc<dyn Fn()>,
    pub(crate) apply: Arc<dyn Fn(String)>,
    pub(crate) pop: Arc<dyn Fn(String)>,
    pub(crate) drop: Arc<dyn Fn(String)>,
}
