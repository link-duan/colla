use cocodec::{Decode, Encode};
use std::collections::BTreeMap;
use std::fmt;

/// A result using the unified public CollaError.
pub type Result<T> = std::result::Result<T, CollaError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
/// Stable error categories shared by Rust, JavaScript and protocol diagnostics.
pub enum ErrorCode {
    #[cocodec(tag = 0)]
    /// An argument violates the public calling contract.
    InvalidArgument,
    #[cocodec(tag = 1)]
    /// Content or an element identity is invalid.
    InvalidValue,
    #[cocodec(tag = 2)]
    /// The typed binary object is malformed or noncanonical.
    InvalidEncoding,
    #[cocodec(tag = 3)]
    /// The runtime or editing scope cannot perform this action.
    InvalidState,
    #[cocodec(tag = 4)]
    /// An input, allocation, sequence or version resource limit was exceeded.
    LimitExceeded,
    #[cocodec(tag = 5)]
    /// The operation or Path segment does not match the target kind.
    TypeMismatch,
    #[cocodec(tag = 6)]
    /// A required Map key or element ID is absent.
    MissingKey,
    #[cocodec(tag = 7)]
    /// A sequence position or range is outside current content.
    OutOfBounds,
    #[cocodec(tag = 8)]
    /// Checked i64 arithmetic overflowed.
    IntegerOverflow,
    #[cocodec(tag = 9)]
    /// Changes cannot be applied or combined against the supplied basis.
    IncompatibleChange,
    #[cocodec(tag = 10)]
    /// A UTF-16 offset splits a surrogate pair.
    InvalidUtf16Boundary,
    #[cocodec(tag = 11)]
    /// Merging would invalidate ownership or require unsafe Map target replacement.
    StructuralConflict,
    #[cocodec(tag = 12)]
    /// A gap must be filled before this server message can be received.
    MissingRevision,
    #[cocodec(tag = 13)]
    /// The Authority no longer retains the requested content basis.
    HistoryExpired,
}

#[derive(Debug, Clone, PartialEq)]
/// Structured public failure with stable classification and diagnostic context.
pub struct CollaError {
    /// Stable machine-readable error classification.
    pub code: ErrorCode,
    /// Public operation associated with the failure.
    pub operation: String,
    /// Diagnostic fields, including a human-readable reason and optional elementId.
    pub details: BTreeMap<String, String>,
}
super::codec::record_codec!(CollaError, code, operation, details);
impl CollaError {
    /// Constructs a structured error with a code and human-readable reason.
    pub fn new(code: ErrorCode, reason: impl Into<String>) -> Self {
        Self {
            code,
            operation: "core".into(),
            details: BTreeMap::from([("reason".into(), reason.into())]),
        }
    }
    /// Adds the public operation name to an error.
    pub fn at(mut self, operation: &str) -> Self {
        self.operation = operation.into();
        self
    }
    /// Adds one diagnostic field to an error.
    pub fn detail(mut self, key: &str, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}
impl fmt::Display for CollaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}): {}",
            self.code.as_str(),
            self.operation,
            self.details.get("reason").map_or("", String::as_str)
        )
    }
}
impl std::error::Error for CollaError {}

impl From<crate::sequence::error::ValueError> for CollaError {
    fn from(e: crate::sequence::error::ValueError) -> Self {
        Self::new(e.code(), e.to_string())
    }
}
impl From<crate::sequence::error::ApplyError> for CollaError {
    fn from(e: crate::sequence::error::ApplyError) -> Self {
        use crate::sequence::error::ApplyError;
        let code = match e {
            ApplyError::TypeMismatch { .. } => ErrorCode::TypeMismatch,
            ApplyError::SequenceOutOfBounds => ErrorCode::OutOfBounds,
            ApplyError::SequenceLengthOverflow => ErrorCode::LimitExceeded,
        };
        Self::new(code, e.to_string())
    }
}

impl ErrorCode {
    /// Returns the stable snake_case error code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidArgument => "invalid_argument",
            Self::InvalidValue => "invalid_value",
            Self::InvalidEncoding => "invalid_encoding",
            Self::InvalidState => "invalid_state",
            Self::LimitExceeded => "limit_exceeded",
            Self::TypeMismatch => "type_mismatch",
            Self::MissingKey => "missing_key",
            Self::OutOfBounds => "out_of_bounds",
            Self::IntegerOverflow => "integer_overflow",
            Self::IncompatibleChange => "incompatible_change",
            Self::InvalidUtf16Boundary => "invalid_utf16_boundary",
            Self::StructuralConflict => "structural_conflict",
            Self::MissingRevision => "missing_revision",
            Self::HistoryExpired => "history_expired",
        }
    }
}

pub(crate) use CollaError as Error;
