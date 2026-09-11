//! Errors used by the private Text and RichText sequence algebra.

use crate::engine::ErrorCode;
use crate::sequence::value::ValueType;
use thiserror::Error;

/// Errors produced while constructing canonical Values and typed Changes.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ValueError {
    /// A Float or floating-point attribute was NaN or infinite.
    #[error("float must be finite")]
    NonFiniteFloat,
    /// An Attrs or AttrPatch input repeated a key.
    #[error("duplicate key: {0}")]
    DuplicateKey(String),
    /// A logical length or allocation capacity exceeded the platform limit.
    #[error("length exceeds the platform limit")]
    LengthOverflow,
}

/// Errors converting between Unicode scalar and UTF-16 Snapshot positions.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Utf16PositionError {
    /// A UTF-16 position exceeded the sequence's UTF-16 length.
    #[error("UTF-16 position {position} is out of bounds (len {len})")]
    Utf16OutOfBounds {
        /// Requested UTF-16 position.
        position: usize,
        /// UTF-16 sequence length.
        len: usize,
    },
    /// A UTF-16 position fell between a surrogate pair's code units.
    #[error("UTF-16 position {position} is inside a surrogate pair")]
    InvalidUtf16Boundary {
        /// Requested UTF-16 position.
        position: usize,
    },
}

/// Errors applying a Change to a concrete Snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ApplyError {
    /// The Change kind did not match the target Value type.
    #[error("type mismatch: expected {expected:?}, got {actual:?}")]
    TypeMismatch {
        /// Value type required by the Change.
        expected: ValueType,
        /// Actual Snapshot Value type.
        actual: ValueType,
    },
    /// A sequence Change consumed beyond the available base sequence.
    #[error("sequence operation consumes beyond the input")]
    SequenceOutOfBounds,
    /// The resulting sequence length or capacity exceeded the platform limit.
    #[error("sequence logical length overflow")]
    SequenceLengthOverflow,
}

/// Errors composing sequential Changes.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ComposeError {
    /// The root Change kinds cannot be composed sequentially.
    #[error("incompatible sequential changes: {left} then {right}")]
    IncompatibleKinds {
        /// First Change kind.
        left: &'static str,
        /// Second Change kind.
        right: &'static str,
    },
    /// Applying a Change to an intermediate replacement failed.
    #[error(transparent)]
    Apply(#[from] ApplyError),
    /// The canonical composed Change exceeded a platform length limit.
    #[error("composed change length exceeds the platform limit")]
    LengthOverflow,
}

/// Errors transforming two concurrent Changes.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum TransformError {
    /// The Changes cannot describe concurrent operations on one Value type.
    #[error("changes cannot share one base value: {left} vs {right}")]
    IncompatibleKinds {
        /// Left Change kind.
        left: &'static str,
        /// Right Change kind.
        right: &'static str,
    },
    /// A transformed Change exceeded a platform length limit.
    #[error("transformed change length exceeds the platform limit")]
    LengthOverflow,
}

/// Errors constructing an inverse Change.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum InvertError {
    /// The Change was not applicable to the supplied original Snapshot.
    #[error(transparent)]
    Apply(#[from] ApplyError),
    /// The inverse Change exceeded a platform length limit.
    #[error("inverse change length exceeds the platform limit")]
    LengthOverflow,
}

impl ValueError {
    /// The stable classification of this error.
    pub fn code(&self) -> ErrorCode {
        match self {
            ValueError::NonFiniteFloat | ValueError::DuplicateKey(_) => ErrorCode::InvalidValue,
            ValueError::LengthOverflow => ErrorCode::LimitExceeded,
        }
    }
}
