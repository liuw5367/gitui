//! Hunk staging bar for the selected file.

use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct HunkActions {
    pub(crate) stage: Arc<dyn Fn(usize)>,
    pub(crate) unstage: Arc<dyn Fn(usize)>,
}
