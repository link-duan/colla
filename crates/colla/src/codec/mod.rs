//! Canonical Value and Change binary encoding.
//!
//! Byte mechanics (varint, zig-zag, strings, floats, tag dispatch, canonical
//! ordering, and structural DoS bounds) come from [`cocodec`]. Byte decoding is
//! bounded solely by cocodec's built-in defenses (a fixed recursion depth and
//! never pre-allocating from an untrusted length); it takes no configurable
//! limits. The body is versionless; applications own protocol envelopes,
//! document identity, authorship, and checksums.

use crate::change::Change;
use crate::error::CodecError;
use crate::value::Value;

/// Encodes a Value into its canonical binary body.
///
/// ```
/// use colla::codec::{decode_value, encode_value};
/// use colla::Value;
///
/// let value = Value::text("hello");
/// let bytes = encode_value(&value);
/// assert_eq!(decode_value(&bytes)?, value);
/// # Ok::<(), colla::CodecError>(())
/// ```
pub fn encode_value(value: &Value) -> Vec<u8> {
    cocodec::encode_to_vec(value).expect("encoding a Value into a Vec is infallible")
}

/// Decodes a canonical Value body.
pub fn decode_value(bytes: &[u8]) -> Result<Value, CodecError> {
    cocodec::decode_from_slice(bytes).map_err(CodecError::from)
}

/// Encodes a Change into its canonical binary body.
pub fn encode_change(change: &Change) -> Vec<u8> {
    cocodec::encode_to_vec(change).expect("encoding a Change into a Vec is infallible")
}

/// Decodes a canonical Change body.
pub fn decode_change(bytes: &[u8]) -> Result<Change, CodecError> {
    cocodec::decode_from_slice(bytes).map_err(CodecError::from)
}

/// Maps a `cocodec` decode error onto colla's error taxonomy.
impl From<cocodec::Error> for CodecError {
    fn from(error: cocodec::Error) -> Self {
        use cocodec::Error as E;
        match error {
            E::UnexpectedEof | E::Io => CodecError::UnexpectedEof { offset: 0 },
            E::TrailingBytes => CodecError::TrailingBytes { offset: 0 },
            E::NonMinimalVarint { offset } => CodecError::NonMinimalVarint { offset },
            E::IntegerOutOfRange { offset } => CodecError::IntegerOutOfRange { offset },
            E::InvalidUtf8 { offset } => CodecError::InvalidUtf8 { offset },
            E::LengthExceedsInput { offset } => CodecError::UnexpectedEof { offset },
            E::NonCanonical { offset, reason } => CodecError::NonCanonical {
                offset,
                context: "codec",
                reason,
            },
            E::NewerSchema { offset, .. } => CodecError::NonCanonical {
                offset,
                context: "codec",
                reason: "unknown newer fields",
            },
            E::UnknownTag {
                offset,
                tag,
                context,
            } => CodecError::UnknownTag {
                offset,
                tag: tag as u8,
                context,
            },
            E::DepthExceeded { limit } => CodecError::LimitExceeded {
                name: "depth",
                actual: limit + 1,
                limit,
            },
            _ => CodecError::UnexpectedEof { offset: 0 },
        }
    }
}

impl Value {
    /// Encodes this Value into its canonical binary body.
    pub fn encode(&self) -> Vec<u8> {
        encode_value(self)
    }

    /// Decodes a canonical Value body.
    pub fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        decode_value(bytes)
    }
}

impl Change {
    /// Encodes this Change into its canonical binary body.
    pub fn encode(&self) -> Vec<u8> {
        encode_change(self)
    }

    /// Decodes a canonical Change body.
    pub fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        decode_change(bytes)
    }
}
