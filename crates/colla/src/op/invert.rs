use std::collections::BTreeMap;

use crate::attrs::{AttrChange, AttrPatch};
use crate::change::{
    Change, ChangeKind, IntChange, ListChange, ListOp, MapChange, MapEntryChange, RichTextChange,
    RichTextOp, TextChange, TextOp,
};
use crate::error::{ApplyError, InvertError};
use crate::path::Path;
use crate::value::{Value, ValueKind, ValueType};

impl Change {
    /// Builds an inverse using the snapshot immediately before this Change.
    pub fn invert(&self, base: &Value) -> Result<Change, InvertError> {
        self.apply_to(base)?;
        invert_change(self, base)
    }
}

fn invert_change(change: &Change, base: &Value) -> Result<Change, InvertError> {
    match change.kind() {
        ChangeKind::Noop => Ok(Change::noop()),
        ChangeKind::Replace(_) => Ok(Change::replace(base.clone())),
        ChangeKind::Int(IntChange::Add(delta)) => Ok(match delta.checked_neg() {
            Some(inverse) => Change::int_add(inverse),
            None => Change::replace(base.clone()),
        }),
        ChangeKind::Map(map_change) => {
            let map = match base.kind() {
                ValueKind::Map(map) => map,
                _ => return Err(type_mismatch(ValueType::Map, base).into()),
            };
            let mut out = BTreeMap::new();
            for (key, entry) in map_change.iter() {
                let inverse = match entry {
                    MapEntryChange::Insert(_) => MapEntryChange::Delete,
                    MapEntryChange::Delete => {
                        MapEntryChange::Insert(map.get(key).expect("validated").clone())
                    }
                    MapEntryChange::Modify(child) => MapEntryChange::Modify(invert_change(
                        child,
                        map.get(key).expect("validated"),
                    )?),
                };
                out.insert(key.clone(), inverse);
            }
            Ok(Change::map(MapChange::from_btree(out)))
        }
        ChangeKind::List(list_change) => {
            let list = match base.kind() {
                ValueKind::List(list) => list,
                _ => return Err(type_mismatch(ValueType::List, base).into()),
            };
            let mut index = 0usize;
            let mut out = Vec::new();
            for op in list_change.ops() {
                match op {
                    ListOp::Retain(len) => {
                        out.push(ListOp::Retain(*len));
                        index += len;
                    }
                    ListOp::Insert(values) => out.push(ListOp::Delete(values.len())),
                    ListOp::Delete(len) => {
                        out.push(ListOp::Insert(list.as_slice()[index..index + len].to_vec()));
                        index += len;
                    }
                    ListOp::Modify(child) => {
                        out.push(ListOp::Modify(invert_change(
                            child,
                            &list.as_slice()[index],
                        )?));
                        index += 1;
                    }
                }
            }
            Ok(Change::list(ListChange::new(out)))
        }
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
            Ok(Change::text(TextChange::new(out)))
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
            Ok(Change::rich_text(RichTextChange::new(out)))
        }
    }
}

fn type_mismatch(expected: ValueType, actual: &Value) -> ApplyError {
    ApplyError::TypeMismatch {
        path: Path::new(),
        expected,
        actual: actual.value_type(),
    }
}
