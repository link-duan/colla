use crate::sequence::change::{Change, ChangeKind, RichTextOp, TextChange, TextOp};
use crate::sequence::error::ApplyError;
use crate::sequence::richtext::{RichSpan, RichText};
use crate::sequence::value::{Text, Value, ValueKind, ValueType};

impl Change {
    /// Applies this Change to `base`, returning a new immutable Value.
    ///
    /// This is the inherent equivalent of [`crate::sequence::op::apply`]. The input is
    /// unchanged on every error path.
    pub fn apply_to(&self, base: &Value) -> Result<Value, ApplyError> {
        apply_sequence(self, base)
    }
}

fn apply_sequence(change: &Change, base: &Value) -> Result<Value, ApplyError> {
    match change.kind() {
        ChangeKind::Noop => Ok(base.clone()),
        ChangeKind::Text(change) => {
            let base_text = match base.kind() {
                ValueKind::Text(text) => text,
                _ => return Err(type_mismatch(ValueType::Text, base)),
            };
            let chars: Vec<char> = base_text.as_str().chars().collect();
            let capacity = text_output_capacity(change, &chars)?;
            let mut out = String::with_capacity(capacity);
            let mut index = 0usize;
            for op in change.ops() {
                match op {
                    TextOp::Retain(len) => {
                        let end = index
                            .checked_add(*len)
                            .ok_or(ApplyError::SequenceOutOfBounds)?;
                        if end > chars.len() {
                            return Err(ApplyError::SequenceOutOfBounds);
                        }
                        out.extend(chars[index..end].iter());
                        index = end;
                    }
                    TextOp::Insert(value) => out.push_str(value),
                    TextOp::Delete(len) => {
                        index = index
                            .checked_add(*len)
                            .ok_or(ApplyError::SequenceOutOfBounds)?;
                        if index > chars.len() {
                            return Err(ApplyError::SequenceOutOfBounds);
                        }
                    }
                }
            }
            out.extend(chars[index..].iter());
            Ok(Value::from_kind(ValueKind::Text(Text::new(out))))
        }
        ChangeKind::RichText(change) => {
            let base_rich = match base.kind() {
                ValueKind::RichText(rich) => rich,
                _ => return Err(type_mismatch(ValueType::RichText, base)),
            };
            let mut cursor = base_rich.cursor();
            let mut out: Vec<RichSpan> = Vec::new();
            for op in change.ops() {
                match op {
                    RichTextOp::Retain { len, attrs } => {
                        if *len > cursor.remaining_len() {
                            return Err(ApplyError::SequenceOutOfBounds);
                        }
                        let mut remaining = *len;
                        while remaining > 0 {
                            let span = cursor.take(remaining).expect("validated RichText range");
                            remaining -= span.len();
                            out.push(RichSpan::from_parts(
                                span.content().clone(),
                                span.attrs().apply_patch(attrs),
                            ));
                        }
                    }
                    RichTextOp::Insert { content, attrs } => {
                        out.push(RichSpan::from_parts(content.clone(), attrs.clone()))
                    }
                    RichTextOp::Delete(len) => {
                        if !cursor.skip(*len) {
                            return Err(ApplyError::SequenceOutOfBounds);
                        }
                    }
                }
            }
            while let Some(span) = cursor.take(cursor.remaining_len()) {
                out.push(span);
            }
            let rich = RichText::from_spans(out).map_err(|_| ApplyError::SequenceLengthOverflow)?;
            Ok(Value::from_kind(ValueKind::RichText(rich)))
        }
    }
}

fn text_output_capacity(change: &TextChange, base: &[char]) -> Result<usize, ApplyError> {
    let mut input = 0usize;
    let mut bytes = 0usize;
    for op in change.ops() {
        match op {
            TextOp::Retain(len) => {
                let end = checked_input_advance(input, *len, base.len())?;
                bytes = add_char_bytes(bytes, &base[input..end])?;
                input = end;
            }
            TextOp::Insert(value) => {
                bytes = checked_output_advance(bytes, value.len())?;
            }
            TextOp::Delete(len) => {
                input = checked_input_advance(input, *len, base.len())?;
            }
        }
    }
    bytes = add_char_bytes(bytes, &base[input..])?;
    if bytes > isize::MAX as usize {
        return Err(ApplyError::SequenceLengthOverflow);
    }
    Ok(bytes)
}

fn add_char_bytes(current: usize, chars: &[char]) -> Result<usize, ApplyError> {
    chars.iter().try_fold(current, |total, character| {
        checked_output_advance(total, character.len_utf8())
    })
}

fn checked_input_advance(current: usize, amount: usize, len: usize) -> Result<usize, ApplyError> {
    let next = current
        .checked_add(amount)
        .ok_or(ApplyError::SequenceOutOfBounds)?;
    if next > len {
        return Err(ApplyError::SequenceOutOfBounds);
    }
    Ok(next)
}

fn checked_output_advance(current: usize, amount: usize) -> Result<usize, ApplyError> {
    current
        .checked_add(amount)
        .ok_or(ApplyError::SequenceLengthOverflow)
}

fn type_mismatch(expected: ValueType, actual: &Value) -> ApplyError {
    ApplyError::TypeMismatch {
        expected,
        actual: actual.value_type(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_capacity_overflow_maps_to_apply_error() {
        let error = checked_output_advance(usize::MAX, 1).unwrap_err();
        assert_eq!(error, ApplyError::SequenceLengthOverflow);
        assert_eq!(
            crate::CollaError::from(error).code,
            crate::ErrorCode::LimitExceeded
        );
    }
}
