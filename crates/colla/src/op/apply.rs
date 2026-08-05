use crate::change::{Change, ChangeKind, IntChange, ListOp, MapEntryChange, RichTextOp, TextOp};
use crate::error::ApplyError;
use crate::path::{Path, PathSeg};
use crate::richtext::{collapse, flatten, RichAtom, RichInsert};
use crate::value::{List, Map, Text, Value, ValueKind, ValueType};

impl Change {
    /// Applies this contextual Change to `base`, returning a new immutable
    /// Value. The input is unchanged on every error path.
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
            let mut out = Vec::new();
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
            let mut out = String::new();
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
            let atoms = flatten(base_rich.spans());
            let mut out: Vec<RichAtom> = Vec::new();
            let mut index = 0usize;
            for op in change.ops() {
                match op {
                    RichTextOp::Retain { len, attrs } => {
                        let end = index.checked_add(*len).ok_or_else(|| {
                            ApplyError::SequenceOutOfBounds { path: path.clone() }
                        })?;
                        if end > atoms.len() {
                            return Err(ApplyError::SequenceOutOfBounds { path: path.clone() });
                        }
                        for atom in &atoms[index..end] {
                            out.push(RichAtom {
                                content: atom.content.clone(),
                                attrs: atom.attrs.apply_patch(attrs),
                            });
                        }
                        index = end;
                    }
                    RichTextOp::Insert { content, attrs } => match content {
                        RichInsert::Text(text) => {
                            for ch in text.chars() {
                                out.push(RichAtom {
                                    content: RichInsert::text(ch.to_string()),
                                    attrs: attrs.clone(),
                                });
                            }
                        }
                        RichInsert::Embed(value) => out.push(RichAtom {
                            content: RichInsert::Embed(value.clone()),
                            attrs: attrs.clone(),
                        }),
                    },
                    RichTextOp::Delete(len) => {
                        index = index.checked_add(*len).ok_or_else(|| {
                            ApplyError::SequenceOutOfBounds { path: path.clone() }
                        })?;
                        if index > atoms.len() {
                            return Err(ApplyError::SequenceOutOfBounds { path: path.clone() });
                        }
                    }
                }
            }
            out.extend_from_slice(&atoms[index..]);
            Ok(Value::from_kind(ValueKind::RichText(collapse(out))))
        }
    }
}

fn type_mismatch(path: &Path, expected: ValueType, actual: &Value) -> ApplyError {
    ApplyError::TypeMismatch {
        path: path.clone(),
        expected,
        actual: actual.value_type(),
    }
}
