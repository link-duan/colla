use crate::change::{
    Change, ListChange, ListOp, MapChange, MapEntryChange, RichTextChange, TextChange, TextOp,
};
use crate::error::{ApplyError, BuildError};
use crate::limits::Limits;
use crate::path::{Path, PathSeg};
use crate::value::{Value, ValueKind, ValueType};

/// Snapshot-aware, sequential construction of one canonical recursive Change.
pub struct ChangeBuilder<'a> {
    limits: &'a Limits,
    current: Value,
    change: Change,
}

impl<'a> ChangeBuilder<'a> {
    pub fn new(base: &Value, limits: &'a Limits) -> Result<Self, ApplyError> {
        base.check_limits(limits)?;
        Ok(Self {
            limits,
            current: base.clone(),
            change: Change::noop(),
        })
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
        let next_value = small.apply_to(&self.current, self.limits)?;
        let next_change = self.change.compose(&small, self.limits)?;
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

    pub fn map_delete(
        &mut self,
        path: &Path,
        key: impl Into<String>,
    ) -> Result<&mut Self, BuildError> {
        let leaf = Change::map(
            MapChange::from_entries([(key.into(), MapEntryChange::Delete)])
                .expect("one unique key"),
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
        self.change_at(
            path,
            Change::list(ListChange::new(vec![
                ListOp::Retain(index),
                ListOp::Delete(len),
            ])),
        )
    }

    pub fn text_insert(
        &mut self,
        path: &Path,
        index: usize,
        value: impl Into<String>,
    ) -> Result<&mut Self, BuildError> {
        self.change_at(
            path,
            Change::text(TextChange::new(vec![
                TextOp::Retain(index),
                TextOp::Insert(value.into()),
            ])),
        )
    }

    pub fn text_delete(
        &mut self,
        path: &Path,
        index: usize,
        len: usize,
    ) -> Result<&mut Self, BuildError> {
        self.change_at(
            path,
            Change::text(TextChange::new(vec![
                TextOp::Retain(index),
                TextOp::Delete(len),
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
