mod apply;
mod compose;
mod invert;
mod reader;
mod transform;

use crate::{ApplyError, Change, ComposeError, InvertError, Value};

pub use transform::transform_pair;

pub fn apply(base: &Value, change: &Change) -> Result<Value, ApplyError> {
    change.apply_to(base)
}

pub fn compose(first: &Change, second: &Change) -> Result<Change, ComposeError> {
    first.compose(second)
}

pub fn invert(change: &Change, base: &Value) -> Result<Change, InvertError> {
    change.invert(base)
}
