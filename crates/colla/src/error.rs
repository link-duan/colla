//! Structured errors returned by Colla's public APIs.

use crate::path::Path;
use crate::value::ValueType;
use thiserror::Error;

/// Errors produced while constructing canonical Values and typed Changes.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ValueError {
    /// A Float or floating-point attribute was NaN or infinite.
    #[error("float must be finite")]
    NonFiniteFloat,
    /// A Map, Attrs, AttrPatch, or MapChange input repeated a key.
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
    /// A Unicode scalar position exceeded the sequence length.
    #[error("code point position {position} is out of bounds (len {len})")]
    CodePointOutOfBounds {
        /// Requested Unicode scalar/embed position.
        position: usize,
        /// Logical sequence length.
        len: usize,
    },
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
    #[error("type mismatch at {path}: expected {expected:?}, got {actual:?}")]
    TypeMismatch {
        /// Snapshot-relative location of the mismatch.
        path: Path,
        /// Value type required by the Change.
        expected: ValueType,
        /// Actual Snapshot Value type.
        actual: ValueType,
    },
    /// A Map operation required a key that was absent.
    #[error("missing map key at {path}: {key}")]
    MissingKey {
        /// Path to the containing Map.
        path: Path,
        /// Required key.
        key: String,
    },
    /// A Map insert targeted a key that already existed.
    #[error("map key already exists at {path}: {key}")]
    ExistingKey {
        /// Path to the containing Map.
        path: Path,
        /// Existing key.
        key: String,
    },
    /// A List operation addressed an element outside the Snapshot.
    #[error("list index {index} out of bounds at {path} (len {len})")]
    IndexOutOfBounds {
        /// Path to the containing List.
        path: Path,
        /// Requested element index.
        index: usize,
        /// Snapshot List length.
        len: usize,
    },
    /// A sequence Change consumed beyond the available base sequence.
    #[error("sequence operation consumes beyond the input at {path}")]
    SequenceOutOfBounds {
        /// Path to the sequence.
        path: Path,
    },
    /// Checked Int addition overflowed `i64`.
    #[error("integer addition overflow at {path}")]
    IntegerOverflow {
        /// Path to the Int Value.
        path: Path,
    },
    /// The resulting sequence length or capacity exceeded the platform limit.
    #[error("sequence logical length overflow at {path}")]
    SequenceLengthOverflow {
        /// Path to the sequence.
        path: Path,
    },
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
    /// Two operations on one Map key cannot be composed.
    #[error("sequential map operations are incompatible for key {0}")]
    IncompatibleMapEntry(String),
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
    /// Map entry operations cannot share one valid base-key state.
    #[error("map entry changes cannot share one base key: {0}")]
    IncompatibleMapEntry(String),
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

/// Errors decoding canonical Value and Change binary bodies.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CodecError {
    /// Input ended before the current value was complete.
    #[error("unexpected end of input at byte {offset}")]
    UnexpectedEof {
        /// Byte offset where more input was required.
        offset: usize,
    },
    /// A tag was not defined for the current codec context.
    #[error("unknown tag 0x{tag:02x} for {context} at byte {offset}")]
    UnknownTag {
        /// Byte offset of the tag.
        offset: usize,
        /// Unknown tag byte.
        tag: u8,
        /// Value, Change, operation, or attribute context.
        context: &'static str,
    },
    /// A varint used more bytes than its canonical shortest encoding.
    #[error("non-minimal varint at byte {offset}")]
    NonMinimalVarint {
        /// Byte offset of the varint.
        offset: usize,
    },
    /// A decoded integer could not fit the required target type.
    #[error("integer is out of range at byte {offset}")]
    IntegerOutOfRange {
        /// Byte offset of the integer.
        offset: usize,
    },
    /// String bytes were not valid UTF-8.
    #[error("invalid UTF-8 at byte {offset}")]
    InvalidUtf8 {
        /// Byte offset of the invalid string.
        offset: usize,
    },
    /// The input represented a structurally valid but non-canonical form.
    #[error("non-canonical {context} at byte {offset}: {reason}")]
    NonCanonical {
        /// Byte offset where non-canonical input was detected.
        offset: usize,
        /// Value, Change, operation, or attribute context.
        context: &'static str,
        /// Stable diagnostic reason within the Rust API.
        reason: &'static str,
    },
    /// Bytes remained after one complete Value or Change.
    #[error("trailing bytes beginning at byte {offset}")]
    TrailingBytes {
        /// Byte offset of the first trailing byte.
        offset: usize,
    },
    /// Untrusted input exceeded one receiver-defined resource limit.
    #[error("resource limit exceeded: {name} ({actual} > {limit})")]
    LimitExceeded {
        /// Resource category.
        name: &'static str,
        /// Observed resource usage.
        actual: usize,
        /// Configured maximum.
        limit: usize,
    },
    /// Decoded content violated a canonical Value construction rule.
    #[error(transparent)]
    Value(#[from] ValueError),
}
