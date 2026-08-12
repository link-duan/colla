//! Functional entry points for Colla's OT algebra.

mod apply;
mod compose;
mod invert;
mod reader;
mod transform;

use crate::{ApplyError, Change, ComposeError, InvertError, Value};

pub use transform::transform_pair;

/// Applies `change` to `base` and returns a new immutable Value.
///
/// Type, key, and sequence-range compatibility are checked against `base`.
pub fn apply(base: &Value, change: &Change) -> Result<Value, ApplyError> {
    change.apply_to(base)
}

/// Composes two sequential Changes into one canonical Change.
///
/// The result has the same effect as applying `first` and then `second`.
pub fn compose(first: &Change, second: &Change) -> Result<Change, ComposeError> {
    first.compose(second)
}

/// Builds the inverse of `change` relative to its original Snapshot.
///
/// A Change does not contain old values, so inversion requires `base`.
pub fn invert(change: &Change, base: &Value) -> Result<Change, InvertError> {
    change.invert(base)
}
