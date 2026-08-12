//! Canonical RichText spans and coordinate conversion.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

use crate::attrs::Attrs;
use crate::error::{Utf16PositionError, ValueError};
use crate::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TextMetrics {
    byte_len: usize,
    scalar_len: usize,
    utf16_len: usize,
}

impl TextMetrics {
    fn new(byte_len: usize, scalar_len: usize, utf16_len: usize) -> Self {
        Self {
            byte_len,
            scalar_len,
            utf16_len,
        }
    }

    fn byte_len(self) -> usize {
        self.byte_len
    }

    fn scalar_len(self) -> usize {
        self.scalar_len
    }

    fn utf16_len(self) -> usize {
        self.utf16_len
    }

    fn checked_add(self, other: Self) -> Result<Self, ValueError> {
        let byte_len = self
            .byte_len
            .checked_add(other.byte_len)
            .filter(|length| *length <= isize::MAX as usize)
            .ok_or(ValueError::LengthOverflow)?;
        let scalar_len = self
            .scalar_len
            .checked_add(other.scalar_len)
            .ok_or(ValueError::LengthOverflow)?;
        let utf16_len = self
            .utf16_len
            .checked_add(other.utf16_len)
            .ok_or(ValueError::LengthOverflow)?;
        Ok(Self::new(byte_len, scalar_len, utf16_len))
    }
}

/// Immutable UTF-8 text content with cached Unicode scalar and UTF-16 lengths.
#[derive(Clone)]
pub struct RichTextChunk {
    text: Arc<str>,
    scalar_len: usize,
    utf16_len: usize,
}

pub(crate) struct TextSlice {
    chunk: RichTextChunk,
    next_byte_offset: usize,
}

impl TextSlice {
    pub(crate) fn into_chunk(self) -> RichTextChunk {
        self.chunk
    }

    pub(crate) fn next_byte_offset(&self) -> usize {
        self.next_byte_offset
    }
}

impl RichTextChunk {
    /// Creates immutable text content and caches its scalar and UTF-16 lengths.
    pub fn new(value: impl Into<String>) -> Self {
        let text: Arc<str> = Arc::from(value.into());
        let byte_len = text.len();
        let (scalar_len, utf16_len) = text.chars().fold((0usize, 0usize), |acc, character| {
            (acc.0 + 1, acc.1 + character.len_utf16())
        });
        Self::from_arc(text, TextMetrics::new(byte_len, scalar_len, utf16_len))
    }

    /// Returns the UTF-8 text content.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns the Unicode scalar length.
    pub fn len(&self) -> usize {
        self.scalar_len
    }

    /// Returns whether the chunk contains no text.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub(crate) fn utf16_len(&self) -> usize {
        self.utf16_len
    }

    fn metrics(&self) -> TextMetrics {
        TextMetrics::new(self.text.len(), self.scalar_len, self.utf16_len)
    }

    pub(crate) fn try_concat(&self, other: &Self) -> Result<Self, ValueError> {
        let metrics = self.metrics().checked_add(other.metrics())?;
        let mut merged = String::with_capacity(metrics.byte_len());
        merged.push_str(self.as_str());
        merged.push_str(other.as_str());
        Ok(Self::from_arc(Arc::from(merged), metrics))
    }

    pub(crate) fn byte_offset_after(&self, byte_offset: usize, scalar_len: usize) -> usize {
        let rest = &self.text[byte_offset..];
        byte_offset + char_prefix_bytes(rest, scalar_len)
    }

    pub(crate) fn slice_prefix_from(&self, byte_offset: usize, scalar_len: usize) -> TextSlice {
        let rest = &self.text[byte_offset..];
        let metrics = char_prefix_metrics(rest, scalar_len);
        debug_assert_eq!(metrics.scalar_len(), scalar_len);
        let next_byte_offset = byte_offset + metrics.byte_len();
        let chunk = if byte_offset == 0 && next_byte_offset == self.text.len() {
            self.clone()
        } else {
            Self::from_arc(
                Arc::from(&self.text[byte_offset..next_byte_offset]),
                metrics,
            )
        };
        TextSlice {
            chunk,
            next_byte_offset,
        }
    }

    fn from_arc(text: Arc<str>, metrics: TextMetrics) -> Self {
        debug_assert_eq!(text.len(), metrics.byte_len());
        Self {
            text,
            scalar_len: metrics.scalar_len(),
            utf16_len: metrics.utf16_len(),
        }
    }
}

impl AsRef<str> for RichTextChunk {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for RichTextChunk {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Debug for RichTextChunk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.text.fmt(formatter)
    }
}

impl PartialEq for RichTextChunk {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl Eq for RichTextChunk {}

impl Hash for RichTextChunk {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

/// Text or one atomic embed carried by a RichText span or insertion.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RichContent {
    /// UTF-8 text addressed by Unicode scalar positions.
    Text(RichTextChunk),
    /// One atomic embedded Core Value with logical length one.
    Embed(Value),
}

impl RichContent {
    /// Creates text content.
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(RichTextChunk::new(value))
    }

    /// Creates one atomic embed.
    pub fn embed(value: Value) -> Self {
        Self::Embed(value)
    }

    /// Returns the logical length in Unicode scalars or atomic embeds.
    pub fn len(&self) -> usize {
        match self {
            Self::Text(value) => value.len(),
            Self::Embed(_) => 1,
        }
    }

    /// Returns whether this is empty text content; embeds are never empty.
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Text(value) if value.is_empty())
    }

    pub(crate) fn utf16_len(&self) -> usize {
        match self {
            Self::Text(value) => value.utf16_len(),
            Self::Embed(_) => 1,
        }
    }
}

/// One attributed text or embed span in a RichText Snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RichSpan {
    content: RichContent,
    attrs: Attrs,
}

impl RichSpan {
    /// Creates a text span.
    pub fn text(value: impl Into<String>, attrs: Attrs) -> Self {
        Self {
            content: RichContent::text(value),
            attrs,
        }
    }

    /// Creates an atomic embed span.
    pub fn embed(value: Value, attrs: Attrs) -> Self {
        Self {
            content: RichContent::embed(value),
            attrs,
        }
    }

    /// Returns the span content.
    pub fn content(&self) -> &RichContent {
        &self.content
    }

    /// Returns the attributes applied to the complete span.
    pub fn attrs(&self) -> &Attrs {
        &self.attrs
    }

    /// Returns the logical scalar/embed length.
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Returns whether this is an empty text span.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub(crate) fn utf16_len(&self) -> usize {
        self.content.utf16_len()
    }

    pub(crate) fn from_parts(content: RichContent, attrs: Attrs) -> Self {
        Self { content, attrs }
    }
}

#[derive(Debug)]
struct RichTextRepr {
    spans: Vec<RichSpan>,
    span_ends: Vec<usize>,
    span_utf16_ends: Vec<usize>,
    len: usize,
    utf16_len: usize,
}

struct RichTextReprBuilder {
    spans: Vec<RichSpan>,
    span_ends: Vec<usize>,
    span_utf16_ends: Vec<usize>,
    len: usize,
    utf16_len: usize,
}

impl RichTextReprBuilder {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            spans: Vec::with_capacity(capacity),
            span_ends: Vec::with_capacity(capacity),
            span_utf16_ends: Vec::with_capacity(capacity),
            len: 0,
            utf16_len: 0,
        }
    }

    fn push(&mut self, span: RichSpan) -> Result<(), ValueError> {
        self.len = self
            .len
            .checked_add(span.len())
            .ok_or(ValueError::LengthOverflow)?;
        self.utf16_len = self
            .utf16_len
            .checked_add(span.utf16_len())
            .ok_or(ValueError::LengthOverflow)?;
        self.spans.push(span);
        self.span_ends.push(self.len);
        self.span_utf16_ends.push(self.utf16_len);
        Ok(())
    }

    fn finish(self) -> RichTextRepr {
        RichTextRepr {
            spans: self.spans,
            span_ends: self.span_ends,
            span_utf16_ends: self.span_utf16_ends,
            len: self.len,
            utf16_len: self.utf16_len,
        }
    }
}

/// Immutable canonical RichText with indexed scalar and UTF-16 positions.
///
/// RichText is a linear sequence of text and atomic embeds. Canonical values
/// contain no empty text spans and merge adjacent text with equal attributes.
#[derive(Clone)]
pub struct RichText(Arc<RichTextRepr>);

impl Default for RichText {
    fn default() -> Self {
        Self::from_spans(Vec::new()).expect("empty RichText is valid")
    }
}

impl fmt::Debug for RichText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("RichText")
            .field(&self.0.spans)
            .finish()
    }
}

impl PartialEq for RichText {
    fn eq(&self, other: &Self) -> bool {
        self.0.spans == other.0.spans
    }
}

impl Eq for RichText {}

impl Hash for RichText {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.spans.hash(state);
    }
}

impl RichText {
    /// Constructs and canonicalizes a RichText value.
    ///
    /// Empty text spans are removed and adjacent text spans with equal
    /// attributes are merged. Length overflow returns
    /// [`ValueError::LengthOverflow`].
    ///
    /// # Examples
    ///
    /// ```
    /// use colla::{Attrs, RichSpan, RichText, Value};
    ///
    /// let rich_text = RichText::from_spans(vec![
    ///     RichSpan::text("Hi", Attrs::new()),
    ///     RichSpan::embed(Value::string("mention:1"), Attrs::new()),
    /// ])?;
    /// assert_eq!(rich_text.len(), 3);
    /// # Ok::<(), colla::ValueError>(())
    /// ```
    pub fn from_spans(spans: Vec<RichSpan>) -> Result<Self, ValueError> {
        let spans = normalize_spans(spans)?;
        let mut builder = RichTextReprBuilder::with_capacity(spans.len());
        for span in spans {
            builder.push(span)?;
        }
        Ok(Self(Arc::new(builder.finish())))
    }

    /// Iterates canonical spans without exposing the physical storage type.
    pub fn iter_spans(&self) -> impl DoubleEndedIterator<Item = &RichSpan> + ExactSizeIterator {
        self.0.spans.iter()
    }

    /// Returns the number of canonical spans.
    pub fn span_count(&self) -> usize {
        self.0.spans.len()
    }

    /// Returns the logical length in Unicode scalars and atomic embeds.
    pub fn len(&self) -> usize {
        self.0.len
    }

    /// Returns whether the sequence contains no text or embeds.
    pub fn is_empty(&self) -> bool {
        self.0.len == 0
    }

    /// Concatenates text spans into a display string and omits embeds.
    pub fn to_plain_string(&self) -> String {
        self.iter_spans()
            .filter_map(|span| match span.content() {
                RichContent::Text(text) => Some(text.as_str()),
                RichContent::Embed(_) => None,
            })
            .collect()
    }

    /// Locates a logical position as `(span index, offset within the span)`.
    /// The end position has no containing span and returns `None`.
    pub fn locate_span(&self, position: usize) -> Option<(usize, usize)> {
        if position >= self.len() {
            return None;
        }
        let span_index = self.0.span_ends.partition_point(|end| *end <= position);
        let span_start = span_index
            .checked_sub(1)
            .map_or(0, |previous| self.0.span_ends[previous]);
        Some((span_index, position - span_start))
    }

    /// Converts a Unicode scalar/embed position to a UTF-16 position.
    ///
    /// The end position is valid. RichText embeds occupy one unit in both
    /// coordinate systems.
    pub fn code_point_to_utf16(&self, position: usize) -> Result<usize, Utf16PositionError> {
        if position > self.len() {
            return Err(Utf16PositionError::CodePointOutOfBounds {
                position,
                len: self.len(),
            });
        }
        if position == self.len() {
            return Ok(self.0.utf16_len);
        }
        let (span_index, span_offset) = self.locate_span(position).expect("position validated");
        let utf16_start = span_index
            .checked_sub(1)
            .map_or(0, |previous| self.0.span_utf16_ends[previous]);
        let span = &self.0.spans[span_index];
        let offset = match span.content() {
            RichContent::Text(text) => char_prefix_metrics(text.as_str(), span_offset).utf16_len(),
            RichContent::Embed(_) => 0,
        };
        Ok(utf16_start + offset)
    }

    /// Converts a UTF-16 position to a Unicode scalar/embed position.
    ///
    /// The end position is valid. A position inside a surrogate pair is rejected.
    pub fn utf16_to_code_point(&self, position: usize) -> Result<usize, Utf16PositionError> {
        if position > self.0.utf16_len {
            return Err(Utf16PositionError::Utf16OutOfBounds {
                position,
                len: self.0.utf16_len,
            });
        }
        if position == self.0.utf16_len {
            return Ok(self.len());
        }
        let span_index = self
            .0
            .span_utf16_ends
            .partition_point(|end| *end <= position);
        let utf16_start = span_index
            .checked_sub(1)
            .map_or(0, |previous| self.0.span_utf16_ends[previous]);
        let scalar_start = span_index
            .checked_sub(1)
            .map_or(0, |previous| self.0.span_ends[previous]);
        let target = position - utf16_start;
        match self.0.spans[span_index].content() {
            RichContent::Embed(_) => Ok(scalar_start),
            RichContent::Text(text) => {
                let mut utf16_offset = 0usize;
                for (scalar_offset, character) in text.chars().enumerate() {
                    if target == utf16_offset {
                        return Ok(scalar_start + scalar_offset);
                    }
                    let next = utf16_offset + character.len_utf16();
                    if target < next {
                        return Err(Utf16PositionError::InvalidUtf16Boundary { position });
                    }
                    utf16_offset = next;
                }
                debug_assert_eq!(target, utf16_offset);
                Ok(scalar_start + text.len())
            }
        }
    }

    pub(crate) fn cursor(&self) -> RichSpanCursor<'_> {
        RichSpanCursor::new(self)
    }
}

fn normalize_spans(spans: Vec<RichSpan>) -> Result<Vec<RichSpan>, ValueError> {
    let mut out: Vec<RichSpan> = Vec::new();
    for span in spans {
        if span.is_empty() {
            continue;
        }
        if let Some(last) = out.last_mut() {
            if spans_are_mergeable(last, &span) {
                let (RichContent::Text(left), RichContent::Text(right)) =
                    (&mut last.content, &span.content)
                else {
                    unreachable!("mergeable RichText spans are text")
                };
                *left = left.try_concat(right)?;
                continue;
            }
        }
        out.push(span);
    }
    Ok(out)
}

fn spans_are_mergeable(left: &RichSpan, right: &RichSpan) -> bool {
    left.attrs == right.attrs
        && matches!(
            (&left.content, &right.content),
            (RichContent::Text(_), RichContent::Text(_))
        )
}

pub(crate) struct RichSpanCursor<'a> {
    rich: &'a RichText,
    span_index: usize,
    scalar_offset: usize,
    byte_offset: usize,
    position: usize,
}

impl<'a> RichSpanCursor<'a> {
    fn new(rich: &'a RichText) -> Self {
        Self {
            rich,
            span_index: 0,
            scalar_offset: 0,
            byte_offset: 0,
            position: 0,
        }
    }

    pub(crate) fn remaining_len(&self) -> usize {
        self.rich.len() - self.position
    }

    pub(crate) fn take(&mut self, max_len: usize) -> Option<RichSpan> {
        if max_len == 0 || self.position == self.rich.len() {
            return None;
        }
        let span = &self.rich.0.spans[self.span_index];
        let take_len = max_len.min(span.len() - self.scalar_offset);
        let (result, byte_advance) = match span.content() {
            RichContent::Text(text) => {
                if self.scalar_offset == 0 && take_len == span.len() {
                    (span.clone(), None)
                } else {
                    let slice = text.slice_prefix_from(self.byte_offset, take_len);
                    let byte_advance = slice.next_byte_offset() - self.byte_offset;
                    (
                        RichSpan::from_parts(
                            RichContent::Text(slice.into_chunk()),
                            span.attrs().clone(),
                        ),
                        Some(byte_advance),
                    )
                }
            }
            RichContent::Embed(value) => {
                debug_assert_eq!(take_len, 1);
                (RichSpan::embed(value.clone(), span.attrs().clone()), None)
            }
        };
        self.advance_within_span(take_len, byte_advance);
        Some(result)
    }

    pub(crate) fn take_attrs(&mut self, max_len: usize) -> Option<(usize, Attrs)> {
        if max_len == 0 || self.position == self.rich.len() {
            return None;
        }
        let span = &self.rich.0.spans[self.span_index];
        let take_len = max_len.min(span.len() - self.scalar_offset);
        let attrs = span.attrs().clone();
        self.advance_within_span(take_len, None);
        Some((take_len, attrs))
    }

    pub(crate) fn skip(&mut self, mut len: usize) -> bool {
        if len > self.remaining_len() {
            return false;
        }
        while len > 0 {
            let span = &self.rich.0.spans[self.span_index];
            let take_len = len.min(span.len() - self.scalar_offset);
            self.advance_within_span(take_len, None);
            len -= take_len;
        }
        true
    }

    fn advance_within_span(&mut self, len: usize, byte_advance: Option<usize>) {
        let span = &self.rich.0.spans[self.span_index];
        let span_remaining = span.len() - self.scalar_offset;
        debug_assert!(len > 0 && len <= span_remaining);
        self.position += len;
        if len == span_remaining {
            self.span_index += 1;
            self.scalar_offset = 0;
            self.byte_offset = 0;
            return;
        }
        if let RichContent::Text(text) = span.content() {
            let advance = byte_advance.unwrap_or_else(|| {
                text.byte_offset_after(self.byte_offset, len) - self.byte_offset
            });
            self.byte_offset += advance;
        }
        self.scalar_offset += len;
    }
}

pub(crate) fn char_prefix_bytes(text: &str, len: usize) -> usize {
    char_prefix_metrics(text, len).byte_len()
}

fn char_prefix_metrics(text: &str, len: usize) -> TextMetrics {
    if len == 0 {
        return TextMetrics::new(0, 0, 0);
    }
    let mut scalar_len = 0usize;
    let mut utf16_len = 0usize;
    for (byte_index, character) in text.char_indices() {
        if scalar_len == len {
            return TextMetrics::new(byte_index, scalar_len, utf16_len);
        }
        scalar_len += 1;
        utf16_len += character.len_utf16();
    }
    TextMetrics::new(text.len(), scalar_len, utf16_len)
}

#[cfg(test)]
mod tests {
    use super::{RichTextChunk, TextMetrics};
    use crate::{Attrs, RichContent, RichTextChange, RichTextOp, ValueError};
    use std::sync::Arc;

    #[test]
    fn text_metrics_reject_all_length_overflows() {
        assert_eq!(
            TextMetrics::new(isize::MAX as usize, 1, 1).checked_add(TextMetrics::new(1, 1, 1)),
            Err(ValueError::LengthOverflow)
        );
        assert_eq!(
            TextMetrics::new(1, usize::MAX, 1).checked_add(TextMetrics::new(1, 1, 1)),
            Err(ValueError::LengthOverflow)
        );
        assert_eq!(
            TextMetrics::new(1, 1, usize::MAX).checked_add(TextMetrics::new(1, 1, 1)),
            Err(ValueError::LengthOverflow)
        );
    }

    #[test]
    fn text_chunk_concat_propagates_cached_length_overflow() {
        let left = RichTextChunk::from_arc(Arc::from("a"), TextMetrics::new(1, usize::MAX, 1));
        let right = RichTextChunk::new("b");

        assert_eq!(left.try_concat(&right), Err(ValueError::LengthOverflow));

        let change = RichTextChange::from_ops([
            RichTextOp::Insert {
                content: RichContent::Text(left),
                attrs: Attrs::new(),
            },
            RichTextOp::Insert {
                content: RichContent::Text(right),
                attrs: Attrs::new(),
            },
        ]);
        assert_eq!(change, Err(ValueError::LengthOverflow));
    }
}
