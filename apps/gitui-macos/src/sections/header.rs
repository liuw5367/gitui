//! Top header: repository input, branch/ahead/behind, sync actions.

use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct HeaderActions {
    pub(crate) open: Arc<dyn Fn()>,
    pub(crate) close: Arc<dyn Fn()>,
    pub(crate) fetch: Arc<dyn Fn()>,
    pub(crate) pull: Arc<dyn Fn()>,
    pub(crate) push: Arc<dyn Fn()>,
}
