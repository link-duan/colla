use crate::path::Path;
use crate::value::ValueType;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ValueError {
    #[error("float must be finite")]
    NonFiniteFloat,
    #[error("duplicate key: {0}")]
    DuplicateKey(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ApplyError {
    #[error("type mismatch at {path}: expected {expected:?}, got {actual:?}")]
    TypeMismatch {
        path: Path,
        expected: ValueType,
        actual: ValueType,
    },
    #[error("missing map key at {path}: {key}")]
    MissingKey { path: Path, key: String },
    #[error("map key already exists at {path}: {key}")]
    ExistingKey { path: Path, key: String },
    #[error("list index {index} out of bounds at {path} (len {len})")]
    IndexOutOfBounds {
        path: Path,
        index: usize,
        len: usize,
    },
    #[error("sequence operation consumes beyond the input at {path}")]
    SequenceOutOfBounds { path: Path },
    #[error("integer addition overflow at {path}")]
    IntegerOverflow { path: Path },
    #[error("resource limit exceeded: {name} ({actual} > {limit})")]
    LimitExceeded {
        name: &'static str,
        actual: usize,
        limit: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ComposeError {
    #[error("incompatible sequential changes: {left} then {right}")]
    IncompatibleKinds {
        left: &'static str,
        right: &'static str,
    },
    #[error("sequential map operations are incompatible for key {0}")]
    IncompatibleMapEntry(String),
    #[error(transparent)]
    Apply(#[from] ApplyError),
    #[error("resource limit exceeded: {name} ({actual} > {limit})")]
    LimitExceeded {
        name: &'static str,
        actual: usize,
        limit: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum TransformError {
    #[error("changes cannot share one base value: {left} vs {right}")]
    IncompatibleKinds {
        left: &'static str,
        right: &'static str,
    },
    #[error("map entry changes cannot share one base key: {0}")]
    IncompatibleMapEntry(String),
    #[error("resource limit exceeded: {name} ({actual} > {limit})")]
    LimitExceeded {
        name: &'static str,
        actual: usize,
        limit: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum InvertError {
    #[error(transparent)]
    Apply(#[from] ApplyError),
    #[error("resource limit exceeded: {name} ({actual} > {limit})")]
    LimitExceeded {
        name: &'static str,
        actual: usize,
        limit: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum BuildError {
    #[error(transparent)]
    Apply(#[from] ApplyError),
    #[error(transparent)]
    Compose(#[from] ComposeError),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CodecError {
    #[error("unexpected end of input at byte {offset}")]
    UnexpectedEof { offset: usize },
    #[error("unknown tag 0x{tag:02x} for {context} at byte {offset}")]
    UnknownTag {
        offset: usize,
        tag: u8,
        context: &'static str,
    },
    #[error("non-minimal varint at byte {offset}")]
    NonMinimalVarint { offset: usize },
    #[error("integer is out of range at byte {offset}")]
    IntegerOutOfRange { offset: usize },
    #[error("invalid UTF-8 at byte {offset}")]
    InvalidUtf8 { offset: usize },
    #[error("non-canonical {context} at byte {offset}: {reason}")]
    NonCanonical {
        offset: usize,
        context: &'static str,
        reason: &'static str,
    },
    #[error("trailing bytes beginning at byte {offset}")]
    TrailingBytes { offset: usize },
    #[error("resource limit exceeded: {name} ({actual} > {limit})")]
    LimitExceeded {
        name: &'static str,
        actual: usize,
        limit: usize,
    },
    #[error(transparent)]
    Value(#[from] ValueError),
}
