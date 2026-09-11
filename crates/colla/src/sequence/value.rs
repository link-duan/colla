//! Scalar values used by the private Text and RichText algebra.

use std::fmt;
use std::sync::Arc;

use crate::sequence::error::{Utf16PositionError, ValueError};
use crate::sequence::richtext::RichText;

/// A finite, canonical `f64` value.
///
/// NaN and infinities are rejected, and negative zero is normalized to
/// positive zero so equality and canonical encoding remain deterministic.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FiniteF64(u64);

impl FiniteF64 {
    /// Creates a canonical finite float.
    pub fn new(value: f64) -> Result<Self, ValueError> {
        if !value.is_finite() {
            return Err(ValueError::NonFiniteFloat);
        }
        let canonical = if value == 0.0 { 0.0 } else { value };
        Ok(Self(canonical.to_bits()))
    }

    /// Returns the represented floating-point value.
    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

impl fmt::Debug for FiniteF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl cocodec::Encode for FiniteF64 {
    fn encode<W: cocodec::Write>(&self, w: &mut W) -> Result<(), cocodec::Error> {
        cocodec::WriteExt::write_f64(w, self.get())
    }
}

impl cocodec::Decode for FiniteF64 {
    fn decode<R: cocodec::Read>(d: &mut cocodec::Decoder<R>) -> Result<Self, cocodec::Error> {
        let offset = cocodec::Decoder::offset(d);
        let value = d.f64()?;
        // FiniteF64::new rejects non-finite and normalizes -0.0 -> +0.0.
        FiniteF64::new(value).map_err(|_| cocodec::Error::NonCanonical {
            offset,
            reason: "non-finite float",
        })
    }
}

/// Collaborative text addressed by Unicode scalar positions.
///
/// Unlike an atomic String Value, Text supports character-level OT.
#[derive(Debug, Clone, PartialEq, Eq, Hash, cocodec::Encode, cocodec::Decode)]
#[cocodec(transparent)]
pub struct Text(Arc<str>);

impl Text {
    /// Creates collaborative text from UTF-8 content.
    pub fn new(value: impl Into<String>) -> Self {
        Self(Arc::from(value.into()))
    }

    /// Returns the UTF-8 text content.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the Unicode scalar length.
    pub fn len(&self) -> usize {
        self.0.chars().count()
    }

    /// Converts a UTF-16 position to a Unicode scalar position.
    ///
    /// The end position is valid. A position inside a surrogate pair is rejected.
    pub fn utf16_to_code_point(&self, position: usize) -> Result<usize, Utf16PositionError> {
        let mut utf16 = 0usize;
        for (code_point, character) in self.0.chars().enumerate() {
            if position == utf16 {
                return Ok(code_point);
            }
            let next = utf16 + character.len_utf16();
            if position < next {
                return Err(Utf16PositionError::InvalidUtf16Boundary { position });
            }
            utf16 = next;
        }
        if position == utf16 {
            Ok(self.len())
        } else {
            Err(Utf16PositionError::Utf16OutOfBounds {
                position,
                len: utf16,
            })
        }
    }
}

/// The sequence kind expected by an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Text,
    RichText,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueKind {
    Text(Text),
    RichText(RichText),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value(ValueKind);
impl Value {
    pub fn text(text: impl Into<String>) -> Self {
        Self(ValueKind::Text(Text::new(text)))
    }
    pub fn rich_text(rich: RichText) -> Self {
        Self(ValueKind::RichText(rich))
    }
    pub fn from_kind(kind: ValueKind) -> Self {
        Self(kind)
    }
    pub fn kind(&self) -> &ValueKind {
        &self.0
    }
    pub fn value_type(&self) -> ValueType {
        match self.0 {
            ValueKind::Text(_) => ValueType::Text,
            ValueKind::RichText(_) => ValueType::RichText,
        }
    }
    pub fn as_text(&self) -> Option<&Text> {
        if let ValueKind::Text(v) = &self.0 {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_rich_text(&self) -> Option<&RichText> {
        if let ValueKind::RichText(v) = &self.0 {
            Some(v)
        } else {
            None
        }
    }
}
