use crate::change::{
    Change, ListChange, ListOp, MapChange, MapEntryChange, RichTextChange, TextChange, TextOp,
};
use crate::error::{ApplyError, BuildError};
use crate::path::{Path, PathSeg};
use crate::value::{Value, ValueKind, ValueType};

/// Snapshot-aware, sequential construction of one canonical recursive Change.
#[derive(Clone)]
pub struct ChangeBuilder {
    current: Value,
    change: Change,
}

impl ChangeBuilder {
    pub fn new(base: &Value) -> Self {
        Self {
            current: base.clone(),
            change: Change::noop(),
        }
    }

    pub fn current(&self) -> &Value {
        &self.current
    }
    pub fn change(&self) -> &Change {
        &self.change
    }
    pub fn build(self) -> Change {
        self.change
    }

    pub fn change_at(&mut self, path: &Path, leaf: Change) -> Result<&mut Self, BuildError> {
        let small = wrap_at(&self.current, path.segments(), path, 0, leaf)?;
        let next_value = small.apply_to(&self.current)?;
        let next_change = self.change.compose(&small)?;
        self.current = next_value;
        self.change = next_change;
        Ok(self)
    }

    pub fn replace(&mut self, path: &Path, value: Value) -> Result<&mut Self, BuildError> {
        self.change_at(path, Change::replace(value))
    }

    pub fn map_insert(
        &mut self,
        path: &Path,
        key: impl Into<String>,
        value: Value,
    ) -> Result<&mut Self, BuildError> {
        let leaf = Change::map(
            MapChange::from_entries([(key.into(), MapEntryChange::Insert(value))])
                .expect("one unique key"),
        );
        self.change_at(path, leaf)
    }

    pub fn map_set(
        &mut self,
        path: &Path,
        key: impl Into<String>,
        value: Value,
    ) -> Result<&mut Self, BuildError> {
        let key = key.into();
        let target = value_at_path(&self.current, path)?;
        let ValueKind::Map(map) = target.kind() else {
            return Err(ApplyError::TypeMismatch {
                path: path.clone(),
                expected: ValueType::Map,
                actual: target.value_type(),
            }
            .into());
        };
        let entry = match map.get(&key) {
            Some(current) if current == &value => return Ok(self),
            Some(_) => MapEntryChange::Modify(Change::replace(value)),
            None => MapEntryChange::Insert(value),
        };
        self.change_at(
            path,
            Change::map(MapChange::from_entries([(key, entry)]).expect("one unique key")),
        )
    }

    pub fn map_delete(
        &mut self,
        path: &Path,
        key: impl Into<String>,
    ) -> Result<&mut Self, BuildError> {
        let key = key.into();
        let target = value_at_path(&self.current, path)?;
        let ValueKind::Map(map) = target.kind() else {
            return Err(ApplyError::TypeMismatch {
                path: path.clone(),
                expected: ValueType::Map,
                actual: target.value_type(),
            }
            .into());
        };
        if map.get(&key).is_none() {
            return Ok(self);
        }
        let leaf = Change::map(
            MapChange::from_entries([(key, MapEntryChange::Delete)]).expect("one unique key"),
        );
        self.change_at(path, leaf)
    }

    pub fn list_insert<I>(
        &mut self,
        path: &Path,
        index: usize,
        values: I,
    ) -> Result<&mut Self, BuildError>
    where
        I: IntoIterator<Item = Value>,
    {
        let values: Vec<Value> = values.into_iter().collect();
        let target = value_at_path(&self.current, path)?;
        let ValueKind::List(list) = target.kind() else {
            return Err(ApplyError::TypeMismatch {
                path: path.clone(),
                expected: ValueType::List,
                actual: target.value_type(),
            }
            .into());
        };
        if index > list.len() {
            return Err(ApplyError::IndexOutOfBounds {
                path: path.clone(),
                index,
                len: list.len(),
            }
            .into());
        }
        if values.is_empty() {
            return Ok(self);
        }
        self.change_at(
            path,
            Change::list(ListChange::new(vec![
                ListOp::Retain(index),
                ListOp::Insert(values),
            ])),
        )
    }

    pub fn list_delete(
        &mut self,
        path: &Path,
        index: usize,
        len: usize,
    ) -> Result<&mut Self, BuildError> {
        let target = value_at_path(&self.current, path)?;
        let ValueKind::List(list) = target.kind() else {
            return Err(ApplyError::TypeMismatch {
                path: path.clone(),
                expected: ValueType::List,
                actual: target.value_type(),
            }
            .into());
        };
        let end = index
            .checked_add(len)
            .ok_or_else(|| ApplyError::IndexOutOfBounds {
                path: path.clone(),
                index,
                len: list.len(),
            })?;
        if end > list.len() {
            return Err(ApplyError::IndexOutOfBounds {
                path: path.clone(),
                index: end,
                len: list.len(),
            }
            .into());
        }
        if len == 0 {
            return Ok(self);
        }
        self.change_at(
            path,
            Change::list(ListChange::new(vec![
                ListOp::Retain(index),
                ListOp::Delete(len),
            ])),
        )
    }

    pub fn list_set(
        &mut self,
        path: &Path,
        index: usize,
        value: Value,
    ) -> Result<&mut Self, BuildError> {
        let target = value_at_path(&self.current, path)?;
        let ValueKind::List(list) = target.kind() else {
            return Err(ApplyError::TypeMismatch {
                path: path.clone(),
                expected: ValueType::List,
                actual: target.value_type(),
            }
            .into());
        };
        let current = list
            .get(index)
            .ok_or_else(|| ApplyError::IndexOutOfBounds {
                path: path.clone(),
                index,
                len: list.len(),
            })?;
        if current == &value {
            return Ok(self);
        }
        self.change_at(
            path,
            Change::list(ListChange::new(vec![
                ListOp::Retain(index),
                ListOp::Modify(Change::replace(value)),
            ])),
        )
    }

    pub fn text_insert(
        &mut self,
        path: &Path,
        index: usize,
        value: impl Into<String>,
    ) -> Result<&mut Self, BuildError> {
        let value = value.into();
        let target = value_at_path(&self.current, path)?;
        let ValueKind::Text(text) = target.kind() else {
            return Err(ApplyError::TypeMismatch {
                path: path.clone(),
                expected: ValueType::Text,
                actual: target.value_type(),
            }
            .into());
        };
        if index > text.len() {
            return Err(ApplyError::IndexOutOfBounds {
                path: path.clone(),
                index,
                len: text.len(),
            }
            .into());
        }
        if value.is_empty() {
            return Ok(self);
        }
        self.change_at(
            path,
            Change::text(TextChange::new(vec![
                TextOp::Retain(index),
                TextOp::Insert(value),
            ])),
        )
    }

    pub fn text_delete(
        &mut self,
        path: &Path,
        index: usize,
        len: usize,
    ) -> Result<&mut Self, BuildError> {
        let target = value_at_path(&self.current, path)?;
        let ValueKind::Text(text) = target.kind() else {
            return Err(ApplyError::TypeMismatch {
                path: path.clone(),
                expected: ValueType::Text,
                actual: target.value_type(),
            }
            .into());
        };
        let end = index
            .checked_add(len)
            .ok_or_else(|| ApplyError::IndexOutOfBounds {
                path: path.clone(),
                index,
                len: text.len(),
            })?;
        if end > text.len() {
            return Err(ApplyError::IndexOutOfBounds {
                path: path.clone(),
                index: end,
                len: text.len(),
            }
            .into());
        }
        if len == 0 {
            return Ok(self);
        }
        self.change_at(
            path,
            Change::text(TextChange::new(vec![
                TextOp::Retain(index),
                TextOp::Delete(len),
            ])),
        )
    }

    pub fn text_replace(
        &mut self,
        path: &Path,
        index: usize,
        len: usize,
        value: impl Into<String>,
    ) -> Result<&mut Self, BuildError> {
        let value = value.into();
        let target = value_at_path(&self.current, path)?;
        let ValueKind::Text(text) = target.kind() else {
            return Err(ApplyError::TypeMismatch {
                path: path.clone(),
                expected: ValueType::Text,
                actual: target.value_type(),
            }
            .into());
        };
        let end = index
            .checked_add(len)
            .ok_or_else(|| ApplyError::IndexOutOfBounds {
                path: path.clone(),
                index,
                len: text.len(),
            })?;
        if end > text.len() {
            return Err(ApplyError::IndexOutOfBounds {
                path: path.clone(),
                index: end,
                len: text.len(),
            }
            .into());
        }
        self.change_at(
            path,
            Change::text(TextChange::new(vec![
                TextOp::Retain(index),
                TextOp::Delete(len),
                TextOp::Insert(value),
            ])),
        )
    }

    pub fn rich_text(
        &mut self,
        path: &Path,
        change: RichTextChange,
    ) -> Result<&mut Self, BuildError> {
        self.change_at(path, Change::rich_text(change))
    }

    pub fn int_add(&mut self, path: &Path, delta: i64) -> Result<&mut Self, BuildError> {
        self.change_at(path, Change::int_add(delta))
    }
}

impl Value {
    /// Starts a sequential ChangeBuilder relative to this Snapshot.
    pub fn change(&self) -> ChangeBuilder {
        ChangeBuilder::new(self)
    }
}

fn wrap_at(
    current: &Value,
    segments: &[PathSeg],
    full_path: &Path,
    depth: usize,
    leaf: Change,
) -> Result<Change, ApplyError> {
    let Some((head, tail)) = segments.split_first() else {
        return Ok(leaf);
    };
    match (current.kind(), head) {
        (ValueKind::Map(map), PathSeg::Key(key)) => {
            let child = map.get(key).ok_or_else(|| ApplyError::MissingKey {
                path: prefix(full_path, depth),
                key: key.clone(),
            })?;
            let nested = wrap_at(child, tail, full_path, depth + 1, leaf)?;
            Ok(Change::map(
                MapChange::from_entries([(key.clone(), MapEntryChange::Modify(nested))])
                    .expect("one unique key"),
            ))
        }
        (ValueKind::List(list), PathSeg::Index(index)) => {
            let child = list
                .get(*index)
                .ok_or_else(|| ApplyError::IndexOutOfBounds {
                    path: prefix(full_path, depth),
                    index: *index,
                    len: list.len(),
                })?;
            let nested = wrap_at(child, tail, full_path, depth + 1, leaf)?;
            Ok(Change::list(ListChange::new(vec![
                ListOp::Retain(*index),
                ListOp::Modify(nested),
            ])))
        }
        (ValueKind::Map(_), PathSeg::Index(_)) => Err(ApplyError::TypeMismatch {
            path: prefix(full_path, depth),
            expected: ValueType::List,
            actual: ValueType::Map,
        }),
        (ValueKind::List(_), PathSeg::Key(_)) => Err(ApplyError::TypeMismatch {
            path: prefix(full_path, depth),
            expected: ValueType::Map,
            actual: ValueType::List,
        }),
        (_, PathSeg::Key(_)) => Err(ApplyError::TypeMismatch {
            path: prefix(full_path, depth),
            expected: ValueType::Map,
            actual: current.value_type(),
        }),
        (_, PathSeg::Index(_)) => Err(ApplyError::TypeMismatch {
            path: prefix(full_path, depth),
            expected: ValueType::List,
            actual: current.value_type(),
        }),
    }
}

fn prefix(path: &Path, len: usize) -> Path {
    let mut out = Path::new();
    for segment in path.segments().iter().take(len) {
        out.push(segment.clone());
    }
    out
}

fn value_at_path<'a>(current: &'a Value, path: &Path) -> Result<&'a Value, ApplyError> {
    let mut value = current;
    for (depth, segment) in path.segments().iter().enumerate() {
        let actual_type = value.value_type();
        value = match (value.kind(), segment) {
            (ValueKind::Map(map), PathSeg::Key(key)) => {
                map.get(key).ok_or_else(|| ApplyError::MissingKey {
                    path: prefix(path, depth),
                    key: key.clone(),
                })?
            }
            (ValueKind::List(list), PathSeg::Index(index)) => {
                list.get(*index)
                    .ok_or_else(|| ApplyError::IndexOutOfBounds {
                        path: prefix(path, depth),
                        index: *index,
                        len: list.len(),
                    })?
            }
            (_, PathSeg::Key(_)) => {
                return Err(ApplyError::TypeMismatch {
                    path: prefix(path, depth),
                    expected: ValueType::Map,
                    actual: actual_type,
                })
            }
            (_, PathSeg::Index(_)) => {
                return Err(ApplyError::TypeMismatch {
                    path: prefix(path, depth),
                    expected: ValueType::List,
                    actual: actual_type,
                })
            }
        };
    }
    Ok(value)
}
