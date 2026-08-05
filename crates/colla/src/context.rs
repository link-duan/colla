use crate::change::{Change, TieBreak};
use crate::codec;
use crate::error::{ApplyError, CodecError, ComposeError, InvertError, TransformError};
use crate::limits::Limits;
use crate::op::transform_pair;
use crate::value::Value;

/// Immutable operation facade that applies one shared set of resource limits.
///
/// `Context` contains no document, session, or mutable application state. The
/// lower-level APIs remain available when per-call limits are more convenient.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    limits: Limits,
}

impl Context {
    /// Creates a facade that uses `limits` for every operation.
    pub fn new(limits: Limits) -> Self {
        Self { limits }
    }

    /// Returns the immutable limits owned by this facade.
    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    /// Applies `change` to `base`.
    pub fn apply(&self, base: &Value, change: &Change) -> Result<Value, ApplyError> {
        change.apply_to(base, &self.limits)
    }

    /// Sequentially composes `first` followed by `second`.
    pub fn compose(&self, first: &Change, second: &Change) -> Result<Change, ComposeError> {
        first.compose(second, &self.limits)
    }

    /// Transforms concurrent changes that share one base value.
    pub fn transform_pair(
        &self,
        left: &Change,
        right: &Change,
        tie_break: TieBreak,
    ) -> Result<(Change, Change), TransformError> {
        transform_pair(left, right, tie_break, &self.limits)
    }

    /// Builds the inverse of `change` relative to `base`.
    pub fn invert(&self, change: &Change, base: &Value) -> Result<Change, InvertError> {
        change.invert(base, &self.limits)
    }

    /// Decodes one canonical Value body.
    pub fn decode_value(&self, bytes: &[u8]) -> Result<Value, CodecError> {
        codec::decode_value(bytes, &self.limits)
    }

    /// Decodes one canonical Change body.
    pub fn decode_change(&self, bytes: &[u8]) -> Result<Change, CodecError> {
        codec::decode_change(bytes, &self.limits)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new(Limits::default())
    }
}
