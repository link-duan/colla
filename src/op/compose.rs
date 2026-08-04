use std::collections::{BTreeMap, VecDeque};

use crate::attrs::AttrPatch;
use crate::change::{
    Change, ChangeKind, IntChange, ListChange, ListOp, MapChange, MapEntryChange, RichTextChange,
    RichTextOp, TextChange, TextOp,
};
use crate::error::ComposeError;
use crate::limits::Limits;
use crate::richtext::RichInsert;

impl Change {
    /// Sequentially composes `self` followed by `next`.
    pub fn compose(&self, next: &Change, limits: &Limits) -> Result<Change, ComposeError> {
        self.check_limits(limits).map_err(map_limit)?;
        next.check_limits(limits).map_err(map_limit)?;
        let result = compose_change(self, next, limits)?;
        result.check_limits(limits).map_err(map_limit)?;
        Ok(result)
    }
}

fn map_limit(error: crate::ApplyError) -> ComposeError {
    match error {
        crate::ApplyError::LimitExceeded {
            name,
            actual,
            limit,
        } => ComposeError::LimitExceeded {
            name,
            actual,
            limit,
        },
        other => ComposeError::Apply(other),
    }
}

fn compose_change(left: &Change, right: &Change, limits: &Limits) -> Result<Change, ComposeError> {
    match (left.kind(), right.kind()) {
        (ChangeKind::Noop, _) => Ok(right.clone()),
        (_, ChangeKind::Noop) => Ok(left.clone()),
        (_, ChangeKind::Replace(value)) => Ok(Change::replace(value.clone())),
        (ChangeKind::Replace(value), _) => Ok(Change::replace(right.apply_to(value, limits)?)),
        (ChangeKind::Int(IntChange::Add(a)), ChangeKind::Int(IntChange::Add(b))) => {
            let sum = a.checked_add(*b).ok_or_else(|| {
                ComposeError::Apply(crate::error::ApplyError::IntegerOverflow {
                    path: crate::Path::new(),
                })
            })?;
            Ok(Change::int_add(sum))
        }
        (ChangeKind::Map(a), ChangeKind::Map(b)) => compose_map(a, b, limits),
        (ChangeKind::List(a), ChangeKind::List(b)) => compose_list(a, b, limits),
        (ChangeKind::Text(a), ChangeKind::Text(b)) => Ok(Change::text(compose_text(a, b))),
        (ChangeKind::RichText(a), ChangeKind::RichText(b)) => {
            Ok(Change::rich_text(compose_rich(a, b)))
        }
        _ => Err(ComposeError::IncompatibleKinds {
            left: left.kind_name(),
            right: right.kind_name(),
        }),
    }
}

fn compose_map(
    left: &MapChange,
    right: &MapChange,
    limits: &Limits,
) -> Result<Change, ComposeError> {
    let mut out = BTreeMap::new();
    let mut keys = BTreeMap::<String, ()>::new();
    for (key, _) in left.iter() {
        keys.insert(key.clone(), ());
    }
    for (key, _) in right.iter() {
        keys.insert(key.clone(), ());
    }
    for key in keys.keys() {
        let a = left
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, value)| value);
        let b = right
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, value)| value);
        let entry = match (a, b) {
            (Some(value), None) => Some(value.clone()),
            (None, Some(value)) => Some(value.clone()),
            (Some(MapEntryChange::Insert(value)), Some(MapEntryChange::Modify(change))) => {
                Some(MapEntryChange::Insert(change.apply_to(value, limits)?))
            }
            (Some(MapEntryChange::Insert(_)), Some(MapEntryChange::Delete)) => None,
            (Some(MapEntryChange::Delete), Some(MapEntryChange::Insert(value))) => {
                Some(MapEntryChange::Modify(Change::replace(value.clone())))
            }
            (Some(MapEntryChange::Modify(a)), Some(MapEntryChange::Modify(b))) => {
                let child = compose_change(a, b, limits)?;
                if child.is_noop() {
                    None
                } else {
                    Some(MapEntryChange::Modify(child))
                }
            }
            (Some(MapEntryChange::Modify(_)), Some(MapEntryChange::Delete)) => {
                Some(MapEntryChange::Delete)
            }
            _ => return Err(ComposeError::IncompatibleMapEntry(key.clone())),
        };
        if let Some(entry) = entry {
            out.insert(key.clone(), entry);
        }
    }
    Ok(Change::map(MapChange::from_btree(out)))
}

#[derive(Clone)]
enum TUnit {
    Retain,
    Insert(char),
    Delete,
}

fn expand_text(change: &TextChange) -> VecDeque<TUnit> {
    let mut out = VecDeque::new();
    for op in change.ops() {
        match op {
            TextOp::Retain(len) => out.extend((0..*len).map(|_| TUnit::Retain)),
            TextOp::Insert(value) => out.extend(value.chars().map(TUnit::Insert)),
            TextOp::Delete(len) => out.extend((0..*len).map(|_| TUnit::Delete)),
        }
    }
    out
}

fn compose_text(left: &TextChange, right: &TextChange) -> TextChange {
    let mut a = expand_text(left);
    let mut b = expand_text(right);
    let mut out = Vec::new();
    while !a.is_empty() || !b.is_empty() {
        if matches!(b.front(), Some(TUnit::Insert(_))) {
            if let Some(TUnit::Insert(ch)) = b.pop_front() {
                out.push(TextOp::Insert(ch.to_string()));
            }
            continue;
        }
        if matches!(a.front(), Some(TUnit::Delete)) {
            a.pop_front();
            out.push(TextOp::Delete(1));
            continue;
        }
        match (a.pop_front(), b.pop_front()) {
            (None, None) => break,
            (None, Some(TUnit::Retain)) => out.push(TextOp::Retain(1)),
            (None, Some(TUnit::Delete)) => out.push(TextOp::Delete(1)),
            (Some(TUnit::Retain), None) => out.push(TextOp::Retain(1)),
            (Some(TUnit::Insert(ch)), None) => out.push(TextOp::Insert(ch.to_string())),
            (Some(TUnit::Insert(ch)), Some(TUnit::Retain)) => {
                out.push(TextOp::Insert(ch.to_string()))
            }
            (Some(TUnit::Insert(_)), Some(TUnit::Delete)) => {}
            (Some(TUnit::Retain), Some(TUnit::Retain)) => out.push(TextOp::Retain(1)),
            (Some(TUnit::Retain), Some(TUnit::Delete)) => out.push(TextOp::Delete(1)),
            _ => unreachable!(),
        }
    }
    TextChange::new(out)
}

#[derive(Clone)]
enum LUnit {
    Retain,
    Insert(crate::Value),
    Delete,
    Modify(Change),
}

fn expand_list(change: &ListChange) -> VecDeque<LUnit> {
    let mut out = VecDeque::new();
    for op in change.ops() {
        match op {
            ListOp::Retain(len) => out.extend((0..*len).map(|_| LUnit::Retain)),
            ListOp::Insert(values) => out.extend(values.iter().cloned().map(LUnit::Insert)),
            ListOp::Delete(len) => out.extend((0..*len).map(|_| LUnit::Delete)),
            ListOp::Modify(change) => out.push_back(LUnit::Modify(change.clone())),
        }
    }
    out
}

fn compose_list(
    left: &ListChange,
    right: &ListChange,
    limits: &Limits,
) -> Result<Change, ComposeError> {
    let mut a = expand_list(left);
    let mut b = expand_list(right);
    let mut out = Vec::new();
    while !a.is_empty() || !b.is_empty() {
        if matches!(b.front(), Some(LUnit::Insert(_))) {
            if let Some(LUnit::Insert(value)) = b.pop_front() {
                out.push(ListOp::Insert(vec![value]));
            }
            continue;
        }
        if matches!(a.front(), Some(LUnit::Delete)) {
            a.pop_front();
            out.push(ListOp::Delete(1));
            continue;
        }
        match (a.pop_front(), b.pop_front()) {
            (None, None) => break,
            (None, Some(LUnit::Retain)) => out.push(ListOp::Retain(1)),
            (None, Some(LUnit::Delete)) => out.push(ListOp::Delete(1)),
            (None, Some(LUnit::Modify(change))) => out.push(ListOp::Modify(change)),
            (Some(LUnit::Retain), None) => out.push(ListOp::Retain(1)),
            (Some(LUnit::Insert(value)), None) => out.push(ListOp::Insert(vec![value])),
            (Some(LUnit::Modify(change)), None) => out.push(ListOp::Modify(change)),
            (Some(LUnit::Insert(value)), Some(LUnit::Retain)) => {
                out.push(ListOp::Insert(vec![value]))
            }
            (Some(LUnit::Insert(_)), Some(LUnit::Delete)) => {}
            (Some(LUnit::Insert(value)), Some(LUnit::Modify(change))) => {
                out.push(ListOp::Insert(vec![change.apply_to(&value, limits)?]))
            }
            (Some(LUnit::Retain), Some(LUnit::Retain)) => out.push(ListOp::Retain(1)),
            (Some(LUnit::Retain), Some(LUnit::Delete))
            | (Some(LUnit::Modify(_)), Some(LUnit::Delete)) => out.push(ListOp::Delete(1)),
            (Some(LUnit::Retain), Some(LUnit::Modify(change))) => out.push(ListOp::Modify(change)),
            (Some(LUnit::Modify(change)), Some(LUnit::Retain)) => out.push(ListOp::Modify(change)),
            (Some(LUnit::Modify(a)), Some(LUnit::Modify(b))) => {
                out.push(ListOp::Modify(compose_change(&a, &b, limits)?))
            }
            _ => unreachable!(),
        }
    }
    Ok(Change::list(ListChange::new(out)))
}

#[derive(Clone)]
enum RUnit {
    Retain(AttrPatch),
    Insert(RichInsert, crate::Attrs),
    Delete,
}

fn expand_rich(change: &RichTextChange) -> VecDeque<RUnit> {
    let mut out = VecDeque::new();
    for op in change.ops() {
        match op {
            RichTextOp::Retain { len, attrs } => {
                out.extend((0..*len).map(|_| RUnit::Retain(attrs.clone())))
            }
            RichTextOp::Insert {
                content: RichInsert::Text(text),
                attrs,
            } => out.extend(
                text.chars()
                    .map(|ch| RUnit::Insert(RichInsert::text(ch.to_string()), attrs.clone())),
            ),
            RichTextOp::Insert { content, attrs } => {
                out.push_back(RUnit::Insert(content.clone(), attrs.clone()))
            }
            RichTextOp::Delete(len) => out.extend((0..*len).map(|_| RUnit::Delete)),
        }
    }
    out
}

fn compose_rich(left: &RichTextChange, right: &RichTextChange) -> RichTextChange {
    let mut a = expand_rich(left);
    let mut b = expand_rich(right);
    let mut out = Vec::new();
    while !a.is_empty() || !b.is_empty() {
        if matches!(b.front(), Some(RUnit::Insert(_, _))) {
            if let Some(RUnit::Insert(content, attrs)) = b.pop_front() {
                out.push(RichTextOp::Insert { content, attrs });
            }
            continue;
        }
        if matches!(a.front(), Some(RUnit::Delete)) {
            a.pop_front();
            out.push(RichTextOp::Delete(1));
            continue;
        }
        match (a.pop_front(), b.pop_front()) {
            (None, None) => break,
            (None, Some(RUnit::Retain(patch))) => out.push(RichTextOp::Retain {
                len: 1,
                attrs: patch,
            }),
            (None, Some(RUnit::Delete)) => out.push(RichTextOp::Delete(1)),
            (Some(RUnit::Retain(patch)), None) => out.push(RichTextOp::Retain {
                len: 1,
                attrs: patch,
            }),
            (Some(RUnit::Insert(content, attrs)), None) => {
                out.push(RichTextOp::Insert { content, attrs })
            }
            (Some(RUnit::Insert(content, attrs)), Some(RUnit::Retain(patch))) => {
                out.push(RichTextOp::Insert {
                    content,
                    attrs: attrs.apply_patch(&patch),
                })
            }
            (Some(RUnit::Insert(_, _)), Some(RUnit::Delete)) => {}
            (Some(RUnit::Retain(a)), Some(RUnit::Retain(b))) => out.push(RichTextOp::Retain {
                len: 1,
                attrs: compose_attr_patch(&a, &b),
            }),
            (Some(RUnit::Retain(_)), Some(RUnit::Delete)) => out.push(RichTextOp::Delete(1)),
            _ => unreachable!(),
        }
    }
    RichTextChange::new(out)
}

pub(crate) fn compose_attr_patch(left: &AttrPatch, right: &AttrPatch) -> AttrPatch {
    let mut out = left.to_btree();
    for (key, value) in right.iter() {
        out.insert(key.clone(), value.clone());
    }
    AttrPatch::from_btree(out)
}
