use std::collections::BTreeMap;

use crate::sequence::attrs::{AttrChange, AttrPatch};
use crate::sequence::change::{Change, ChangeKind, RichTextChange, RichTextOp, TextChange, TextOp};
use crate::sequence::error::{ApplyError, InvertError};
use crate::sequence::value::{Value, ValueKind, ValueType};

impl Change {
    /// Builds an inverse using the Snapshot immediately before this Change.
    ///
    /// This is the inherent equivalent of [`crate::sequence::op::invert`].
    pub fn invert(&self, base: &Value) -> Result<Change, InvertError> {
        self.apply_to(base)?;
        invert_change(self, base)
    }
}

fn invert_change(change: &Change, base: &Value) -> Result<Change, InvertError> {
    match change.kind() {
        ChangeKind::Noop => Ok(Change::noop()),
        ChangeKind::Text(text_change) => {
            let text = match base.kind() {
                ValueKind::Text(text) => text,
                _ => return Err(type_mismatch(ValueType::Text, base).into()),
            };
            let chars: Vec<char> = text.as_str().chars().collect();
            let mut index = 0usize;
            let mut out = Vec::new();
            for op in text_change.ops() {
                match op {
                    TextOp::Retain(len) => {
                        out.push(TextOp::Retain(*len));
                        index += len;
                    }
                    TextOp::Insert(value) => out.push(TextOp::Delete(value.chars().count())),
                    TextOp::Delete(len) => {
                        out.push(TextOp::Insert(chars[index..index + len].iter().collect()));
                        index += len;
                    }
                }
            }
            TextChange::from_ops(out)
                .map(Change::from)
                .map_err(|_| InvertError::LengthOverflow)
        }
        ChangeKind::RichText(rich_change) => {
            let rich = match base.kind() {
                ValueKind::RichText(rich) => rich,
                _ => return Err(type_mismatch(ValueType::RichText, base).into()),
            };
            let mut cursor = rich.cursor();
            let mut out = Vec::new();
            for op in rich_change.ops() {
                match op {
                    RichTextOp::Retain { len, attrs } => {
                        let mut remaining = *len;
                        while remaining > 0 {
                            let (span_len, span_attrs) = cursor
                                .take_attrs(remaining)
                                .expect("Change validated by apply");
                            remaining -= span_len;
                            let mut inverse = BTreeMap::new();
                            for (key, _) in attrs.iter() {
                                inverse.insert(
                                    key.clone(),
                                    match span_attrs.get(key) {
                                        Some(value) => AttrChange::Set(value.clone()),
                                        None => AttrChange::Remove,
                                    },
                                );
                            }
                            out.push(RichTextOp::Retain {
                                len: span_len,
                                attrs: AttrPatch::from_btree(inverse),
                            });
                        }
                    }
                    RichTextOp::Insert { content, .. } => {
                        out.push(RichTextOp::Delete(content.len()))
                    }
                    RichTextOp::Delete(len) => {
                        let mut remaining = *len;
                        while remaining > 0 {
                            let span = cursor.take(remaining).expect("Change validated by apply");
                            remaining -= span.len();
                            out.push(RichTextOp::Insert {
                                content: span.content().clone(),
                                attrs: span.attrs().clone(),
                            });
                        }
                    }
                }
            }
            RichTextChange::from_ops(out)
                .map(Change::from)
                .map_err(|_| InvertError::LengthOverflow)
        }
    }
}

fn type_mismatch(expected: ValueType, actual: &Value) -> ApplyError {
    ApplyError::TypeMismatch {
        expected,
        actual: actual.value_type(),
    }
}
