use crate::change::{
    Change, ChangeKind, IntChange, ListChange, ListOp, MapEntryChange, RichTextOp, TextChange,
    TextOp,
};
use crate::error::ApplyError;
use crate::path::{Path, PathSeg};
use crate::richtext::{RichSpan, RichText};
use crate::value::{List, Map, Text, Value, ValueKind, ValueType};

impl Change {
    /// Applies this Change to `base`, returning a new immutable Value.
    ///
    /// This is the inherent equivalent of [`crate::apply`]. The input is
    /// unchanged on every error path.
    pub fn apply_to(&self, base: &Value) -> Result<Value, ApplyError> {
        let mut path = Path::new();
        apply_at(self, base, &mut path)
    }
}

fn apply_at(change: &Change, base: &Value, path: &mut Path) -> Result<Value, ApplyError> {
    match change.kind() {
        ChangeKind::Noop => Ok(base.clone()),
        ChangeKind::Replace(value) => Ok(value.clone()),
        ChangeKind::Int(IntChange::Add(delta)) => {
            let value = match base.kind() {
                ValueKind::Int(value) => *value,
                _ => return Err(type_mismatch(path, ValueType::Int, base)),
            };
            value
                .checked_add(*delta)
                .map(Value::int)
                .ok_or_else(|| ApplyError::IntegerOverflow { path: path.clone() })
        }
        ChangeKind::Map(change) => {
            let base_map = match base.kind() {
                ValueKind::Map(map) => map,
                _ => return Err(type_mismatch(path, ValueType::Map, base)),
            };
            let mut out = base_map.to_btree();
            for (key, entry) in change.iter() {
                path.push(PathSeg::Key(key.clone()));
                match entry {
                    MapEntryChange::Insert(value) => {
                        if out.contains_key(key) {
                            path.pop();
                            return Err(ApplyError::ExistingKey {
                                path: path.clone(),
                                key: key.clone(),
                            });
                        }
                        out.insert(key.clone(), value.clone());
                    }
                    MapEntryChange::Delete => {
                        if out.remove(key).is_none() {
                            path.pop();
                            return Err(ApplyError::MissingKey {
                                path: path.clone(),
                                key: key.clone(),
                            });
                        }
                    }
                    MapEntryChange::Modify(child) => {
                        let old = match out.get(key) {
                            Some(value) => value.clone(),
                            None => {
                                path.pop();
                                return Err(ApplyError::MissingKey {
                                    path: path.clone(),
                                    key: key.clone(),
                                });
                            }
                        };
                        let new = apply_at(child, &old, path)?;
                        out.insert(key.clone(), new);
                    }
                }
                path.pop();
            }
            Ok(Value::from_kind(ValueKind::Map(Map::from_btree(out))))
        }
        ChangeKind::List(change) => {
            let base_list = match base.kind() {
                ValueKind::List(list) => list,
                _ => return Err(type_mismatch(path, ValueType::List, base)),
            };
            let capacity = list_output_capacity(change, base_list.len(), path)?;
            let mut out = Vec::with_capacity(capacity);
            let mut index = 0usize;
            for op in change.ops() {
                match op {
                    ListOp::Retain(len) => {
                        let end = index.checked_add(*len).ok_or_else(|| {
                            ApplyError::SequenceOutOfBounds { path: path.clone() }
                        })?;
                        if end > base_list.len() {
                            return Err(ApplyError::SequenceOutOfBounds { path: path.clone() });
                        }
                        out.extend_from_slice(&base_list.as_slice()[index..end]);
                        index = end;
                    }
                    ListOp::Insert(values) => out.extend(values.iter().cloned()),
                    ListOp::Delete(len) => {
                        index = index.checked_add(*len).ok_or_else(|| {
                            ApplyError::SequenceOutOfBounds { path: path.clone() }
                        })?;
                        if index > base_list.len() {
                            return Err(ApplyError::SequenceOutOfBounds { path: path.clone() });
                        }
                    }
                    ListOp::Modify(child) => {
                        let old =
                            base_list
                                .get(index)
                                .ok_or_else(|| ApplyError::IndexOutOfBounds {
                                    path: path.clone(),
                                    index,
                                    len: base_list.len(),
                                })?;
                        path.push(PathSeg::Index(index));
                        let new = apply_at(child, old, path)?;
                        path.pop();
                        out.push(new);
                        index += 1;
                    }
                }
            }
            out.extend_from_slice(&base_list.as_slice()[index..]);
            Ok(Value::from_kind(ValueKind::List(List::new(out))))
        }
        ChangeKind::Text(change) => {
            let base_text = match base.kind() {
                ValueKind::Text(text) => text,
                _ => return Err(type_mismatch(path, ValueType::Text, base)),
            };
            let chars: Vec<char> = base_text.as_str().chars().collect();
            let capacity = text_output_capacity(change, &chars, path)?;
            let mut out = String::with_capacity(capacity);
            let mut index = 0usize;
            for op in change.ops() {
                match op {
                    TextOp::Retain(len) => {
                        let end = index.checked_add(*len).ok_or_else(|| {
                            ApplyError::SequenceOutOfBounds { path: path.clone() }
                        })?;
                        if end > chars.len() {
                            return Err(ApplyError::SequenceOutOfBounds { path: path.clone() });
                        }
                        out.extend(chars[index..end].iter());
                        index = end;
                    }
                    TextOp::Insert(value) => out.push_str(value),
                    TextOp::Delete(len) => {
                        index = index.checked_add(*len).ok_or_else(|| {
                            ApplyError::SequenceOutOfBounds { path: path.clone() }
                        })?;
                        if index > chars.len() {
                            return Err(ApplyError::SequenceOutOfBounds { path: path.clone() });
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
                _ => return Err(type_mismatch(path, ValueType::RichText, base)),
            };
            let mut cursor = base_rich.cursor();
            let mut out: Vec<RichSpan> = Vec::new();
            for op in change.ops() {
                match op {
                    RichTextOp::Retain { len, attrs } => {
                        if *len > cursor.remaining_len() {
                            return Err(ApplyError::SequenceOutOfBounds { path: path.clone() });
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
                            return Err(ApplyError::SequenceOutOfBounds { path: path.clone() });
                        }
                    }
                }
            }
            while let Some(span) = cursor.take(cursor.remaining_len()) {
                out.push(span);
            }
            let rich = RichText::from_spans(out)
                .map_err(|_| ApplyError::SequenceLengthOverflow { path: path.clone() })?;
            Ok(Value::from_kind(ValueKind::RichText(rich)))
        }
    }
}

fn list_output_capacity(
    change: &ListChange,
    base_len: usize,
    path: &Path,
) -> Result<usize, ApplyError> {
    let mut input = 0usize;
    let mut output = 0usize;
    for op in change.ops() {
        match op {
            ListOp::Retain(len) => {
                input = checked_input_advance(input, *len, base_len, path)?;
                output = checked_output_advance(output, *len, path)?;
            }
            ListOp::Insert(values) => {
                output = checked_output_advance(output, values.len(), path)?;
            }
            ListOp::Delete(len) => {
                input = checked_input_advance(input, *len, base_len, path)?;
            }
            ListOp::Modify(_) => {
                input = checked_input_advance(input, 1, base_len, path)?;
                output = checked_output_advance(output, 1, path)?;
            }
        }
    }
    output = checked_output_advance(output, base_len - input, path)?;
    let element_size = std::mem::size_of::<Value>();
    if element_size != 0 && output > isize::MAX as usize / element_size {
        return Err(ApplyError::SequenceLengthOverflow { path: path.clone() });
    }
    Ok(output)
}

fn text_output_capacity(
    change: &TextChange,
    base: &[char],
    path: &Path,
) -> Result<usize, ApplyError> {
    let mut input = 0usize;
    let mut bytes = 0usize;
    for op in change.ops() {
        match op {
            TextOp::Retain(len) => {
                let end = checked_input_advance(input, *len, base.len(), path)?;
                bytes = add_char_bytes(bytes, &base[input..end], path)?;
                input = end;
            }
            TextOp::Insert(value) => {
                bytes = checked_output_advance(bytes, value.len(), path)?;
            }
            TextOp::Delete(len) => {
                input = checked_input_advance(input, *len, base.len(), path)?;
            }
        }
    }
    bytes = add_char_bytes(bytes, &base[input..], path)?;
    if bytes > isize::MAX as usize {
        return Err(ApplyError::SequenceLengthOverflow { path: path.clone() });
    }
    Ok(bytes)
}

fn add_char_bytes(current: usize, chars: &[char], path: &Path) -> Result<usize, ApplyError> {
    chars.iter().try_fold(current, |total, character| {
        checked_output_advance(total, character.len_utf8(), path)
    })
}

fn checked_input_advance(
    current: usize,
    amount: usize,
    len: usize,
    path: &Path,
) -> Result<usize, ApplyError> {
    let next = current
        .checked_add(amount)
        .ok_or_else(|| ApplyError::SequenceOutOfBounds { path: path.clone() })?;
    if next > len {
        return Err(ApplyError::SequenceOutOfBounds { path: path.clone() });
    }
    Ok(next)
}

fn checked_output_advance(current: usize, amount: usize, path: &Path) -> Result<usize, ApplyError> {
    current
        .checked_add(amount)
        .ok_or_else(|| ApplyError::SequenceLengthOverflow { path: path.clone() })
}

fn type_mismatch(path: &Path, expected: ValueType, actual: &Value) -> ApplyError {
    ApplyError::TypeMismatch {
        path: path.clone(),
        expected,
        actual: actual.value_type(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_capacity_overflow_maps_to_apply_error() {
        assert_eq!(
            checked_output_advance(usize::MAX, 1, &Path::new()),
            Err(ApplyError::SequenceLengthOverflow { path: Path::new() })
        );
    }
}
