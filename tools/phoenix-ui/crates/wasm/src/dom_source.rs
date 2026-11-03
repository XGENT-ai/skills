//! The [`Dom`] the exports run against: the live-page probe ([`JsDom`]).

use crate::js_dom::JsDom;
use impeccable_core::browser::Dom;

/// Run `f` against the live-page probe.
pub fn with_dom<R>(f: impl FnOnce(&dyn Dom) -> R) -> R {
    f(&JsDom::fresh())
}
