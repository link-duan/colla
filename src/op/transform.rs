use std::collections::{BTreeMap, VecDeque};

use crate::attrs::AttrPatch;
use crate::change::{
    Change, ChangeKind, ListChange, ListOp, MapChange, MapEntryChange, RichTextChange, RichTextOp,
    TextChange, TextOp, TieBreak,
};
use crate::error::TransformError;
use crate::limits::Limits;
use crate::richtext::RichInsert;

/// Transforms concurrent Changes based on one shared snapshot.
///
/// The returned `(left_prime, right_prime)` satisfies TP1 whenever both
/// transformed execution paths are applicable:
/// `apply(apply(base, left), right_prime) ==
/// apply(apply(base, right), left_prime)`.
pub fn transform_pair(
    left: &Change,
    right: &Change,
    tie_break: TieBreak,
    limits: &Limits,
) -> Result<(Change, Change), TransformError> {
    for change in [left, right] {
        if let Err(crate::ApplyError::LimitExceeded {
            name,
            actual,
            limit,
        }) = change.check_limits(limits)
        {
            return Err(TransformError::LimitExceeded {
                name,
                actual,
                limit,
            });
        }
    }
    let pair = transform_change(left, right, tie_break, limits)?;
    for change in [&pair.0, &pair.1] {
        if let Err(crate::ApplyError::LimitExceeded {
            name,
            actual,
            limit,
        }) = change.check_limits(limits)
        {
            return Err(TransformError::LimitExceeded {
                name,
                actual,
                limit,
            });
        }
    }
    Ok(pair)
}

fn transform_change(
    left: &Change,
    right: &Change,
    tie: TieBreak,
    limits: &Limits,
) -> Result<(Change, Change), TransformError> {
    match (left.kind(), right.kind()) {
        (ChangeKind::Noop, _) => Ok((Change::noop(), right.clone())),
        (_, ChangeKind::Noop) => Ok((left.clone(), Change::noop())),
        (ChangeKind::Replace(_), ChangeKind::Replace(_)) => match tie {
            TieBreak::LeftFirst => Ok((left.clone(), Change::noop())),
            TieBreak::RightFirst => Ok((Change::noop(), right.clone())),
        },
        (ChangeKind::Replace(_), _) => Ok((left.clone(), Change::noop())),
        (_, ChangeKind::Replace(_)) => Ok((Change::noop(), right.clone())),
        (ChangeKind::Int(_), ChangeKind::Int(_)) => Ok((left.clone(), right.clone())),
        (ChangeKind::Map(a), ChangeKind::Map(b)) => transform_map(a, b, tie, limits),
        (ChangeKind::List(a), ChangeKind::List(b)) => transform_list(a, b, tie, limits),
        (ChangeKind::Text(a), ChangeKind::Text(b)) => {
            let (a, b) = transform_text(a, b, tie);
            Ok((Change::text(a), Change::text(b)))
        }
        (ChangeKind::RichText(a), ChangeKind::RichText(b)) => {
            let (a, b) = transform_rich(a, b, tie);
            Ok((Change::rich_text(a), Change::rich_text(b)))
        }
        _ => Err(TransformError::IncompatibleKinds {
            left: left.kind_name(),
            right: right.kind_name(),
        }),
    }
}

fn transform_map(
    left: &MapChange,
    right: &MapChange,
    tie: TieBreak,
    limits: &Limits,
) -> Result<(Change, Change), TransformError> {
    let mut a_out = BTreeMap::new();
    let mut b_out = BTreeMap::new();
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
        match (a, b) {
            (Some(value), None) => {
                a_out.insert(key.clone(), value.clone());
            }
            (None, Some(value)) => {
                b_out.insert(key.clone(), value.clone());
            }
            (Some(MapEntryChange::Insert(a)), Some(MapEntryChange::Insert(b))) => match tie {
                TieBreak::LeftFirst => {
                    a_out.insert(
                        key.clone(),
                        MapEntryChange::Modify(Change::replace(a.clone())),
                    );
                }
                TieBreak::RightFirst => {
                    b_out.insert(
                        key.clone(),
                        MapEntryChange::Modify(Change::replace(b.clone())),
                    );
                }
            },
            (Some(MapEntryChange::Delete), Some(MapEntryChange::Delete)) => {}
            (Some(MapEntryChange::Delete), Some(MapEntryChange::Modify(_))) => {
                a_out.insert(key.clone(), MapEntryChange::Delete);
            }
            (Some(MapEntryChange::Modify(_)), Some(MapEntryChange::Delete)) => {
                b_out.insert(key.clone(), MapEntryChange::Delete);
            }
            (Some(MapEntryChange::Modify(a)), Some(MapEntryChange::Modify(b))) => {
                let (ap, bp) = transform_change(a, b, tie, limits)?;
                if !ap.is_noop() {
                    a_out.insert(key.clone(), MapEntryChange::Modify(ap));
                }
                if !bp.is_noop() {
                    b_out.insert(key.clone(), MapEntryChange::Modify(bp));
                }
            }
            _ => return Err(TransformError::IncompatibleMapEntry(key.clone())),
        }
    }
    Ok((
        Change::map(MapChange::from_btree(a_out)),
        Change::map(MapChange::from_btree(b_out)),
    ))
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
            TextOp::Retain(n) => out.extend((0..*n).map(|_| TUnit::Retain)),
            TextOp::Insert(s) => out.extend(s.chars().map(TUnit::Insert)),
            TextOp::Delete(n) => out.extend((0..*n).map(|_| TUnit::Delete)),
        }
    }
    out
}
fn transform_text(
    left: &TextChange,
    right: &TextChange,
    tie: TieBreak,
) -> (TextChange, TextChange) {
    let mut a = expand_text(left);
    let mut b = expand_text(right);
    let mut ao = Vec::new();
    let mut bo = Vec::new();
    while !a.is_empty() || !b.is_empty() {
        let both_insert = matches!(a.front(), Some(TUnit::Insert(_)))
            && matches!(b.front(), Some(TUnit::Insert(_)));
        if matches!(a.front(), Some(TUnit::Insert(_)))
            && (!both_insert || tie == TieBreak::LeftFirst)
        {
            if let Some(TUnit::Insert(ch)) = a.pop_front() {
                ao.push(TextOp::Insert(ch.to_string()));
                bo.push(TextOp::Retain(1));
            }
            continue;
        }
        if matches!(b.front(), Some(TUnit::Insert(_))) {
            if let Some(TUnit::Insert(ch)) = b.pop_front() {
                ao.push(TextOp::Retain(1));
                bo.push(TextOp::Insert(ch.to_string()));
            }
            continue;
        }
        match (a.pop_front(), b.pop_front()) {
            (None, None) => break,
            (Some(TUnit::Retain), None) => ao.push(TextOp::Retain(1)),
            (Some(TUnit::Delete), None) => ao.push(TextOp::Delete(1)),
            (None, Some(TUnit::Retain)) => bo.push(TextOp::Retain(1)),
            (None, Some(TUnit::Delete)) => bo.push(TextOp::Delete(1)),
            (Some(TUnit::Delete), Some(TUnit::Delete)) => {}
            (Some(TUnit::Delete), Some(TUnit::Retain)) => ao.push(TextOp::Delete(1)),
            (Some(TUnit::Retain), Some(TUnit::Delete)) => bo.push(TextOp::Delete(1)),
            (Some(TUnit::Retain), Some(TUnit::Retain)) => {
                ao.push(TextOp::Retain(1));
                bo.push(TextOp::Retain(1));
            }
            _ => unreachable!(),
        }
    }
    (TextChange::new(ao), TextChange::new(bo))
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
            ListOp::Retain(n) => out.extend((0..*n).map(|_| LUnit::Retain)),
            ListOp::Insert(values) => out.extend(values.iter().cloned().map(LUnit::Insert)),
            ListOp::Delete(n) => out.extend((0..*n).map(|_| LUnit::Delete)),
            ListOp::Modify(change) => out.push_back(LUnit::Modify(change.clone())),
        }
    }
    out
}
fn transform_list(
    left: &ListChange,
    right: &ListChange,
    tie: TieBreak,
    limits: &Limits,
) -> Result<(Change, Change), TransformError> {
    let mut a = expand_list(left);
    let mut b = expand_list(right);
    let mut ao = Vec::new();
    let mut bo = Vec::new();
    while !a.is_empty() || !b.is_empty() {
        let both_insert = matches!(a.front(), Some(LUnit::Insert(_)))
            && matches!(b.front(), Some(LUnit::Insert(_)));
        if matches!(a.front(), Some(LUnit::Insert(_)))
            && (!both_insert || tie == TieBreak::LeftFirst)
        {
            if let Some(LUnit::Insert(value)) = a.pop_front() {
                ao.push(ListOp::Insert(vec![value]));
                bo.push(ListOp::Retain(1));
            }
            continue;
        }
        if matches!(b.front(), Some(LUnit::Insert(_))) {
            if let Some(LUnit::Insert(value)) = b.pop_front() {
                ao.push(ListOp::Retain(1));
                bo.push(ListOp::Insert(vec![value]));
            }
            continue;
        }
        match (a.pop_front(), b.pop_front()) {
            (None, None) => break,
            (Some(LUnit::Retain), None) => ao.push(ListOp::Retain(1)),
            (Some(LUnit::Delete), None) => ao.push(ListOp::Delete(1)),
            (Some(LUnit::Modify(change)), None) => ao.push(ListOp::Modify(change)),
            (None, Some(LUnit::Retain)) => bo.push(ListOp::Retain(1)),
            (None, Some(LUnit::Delete)) => bo.push(ListOp::Delete(1)),
            (None, Some(LUnit::Modify(change))) => bo.push(ListOp::Modify(change)),
            (Some(LUnit::Delete), Some(LUnit::Delete)) => {}
            (Some(LUnit::Delete), Some(LUnit::Retain | LUnit::Modify(_))) => {
                ao.push(ListOp::Delete(1))
            }
            (Some(LUnit::Retain | LUnit::Modify(_)), Some(LUnit::Delete)) => {
                bo.push(ListOp::Delete(1))
            }
            (Some(LUnit::Retain), Some(LUnit::Retain)) => {
                ao.push(ListOp::Retain(1));
                bo.push(ListOp::Retain(1));
            }
            (Some(LUnit::Modify(change)), Some(LUnit::Retain)) => {
                ao.push(ListOp::Modify(change));
                bo.push(ListOp::Retain(1));
            }
            (Some(LUnit::Retain), Some(LUnit::Modify(change))) => {
                ao.push(ListOp::Retain(1));
                bo.push(ListOp::Modify(change));
            }
            (Some(LUnit::Modify(a)), Some(LUnit::Modify(b))) => {
                let (ap, bp) = transform_change(&a, &b, tie, limits)?;
                ao.push(if ap.is_noop() {
                    ListOp::Retain(1)
                } else {
                    ListOp::Modify(ap)
                });
                bo.push(if bp.is_noop() {
                    ListOp::Retain(1)
                } else {
                    ListOp::Modify(bp)
                });
            }
            _ => unreachable!(),
        }
    }
    Ok((
        Change::list(ListChange::new(ao)),
        Change::list(ListChange::new(bo)),
    ))
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
            RichTextOp::Delete(n) => out.extend((0..*n).map(|_| RUnit::Delete)),
        }
    }
    out
}
fn transform_rich(
    left: &RichTextChange,
    right: &RichTextChange,
    tie: TieBreak,
) -> (RichTextChange, RichTextChange) {
    let mut a = expand_rich(left);
    let mut b = expand_rich(right);
    let mut ao = Vec::new();
    let mut bo = Vec::new();
    while !a.is_empty() || !b.is_empty() {
        let both_insert = matches!(a.front(), Some(RUnit::Insert(_, _)))
            && matches!(b.front(), Some(RUnit::Insert(_, _)));
        if matches!(a.front(), Some(RUnit::Insert(_, _)))
            && (!both_insert || tie == TieBreak::LeftFirst)
        {
            if let Some(RUnit::Insert(content, attrs)) = a.pop_front() {
                let len = content.len();
                ao.push(RichTextOp::Insert { content, attrs });
                bo.push(RichTextOp::Retain {
                    len,
                    attrs: AttrPatch::new(),
                });
            }
            continue;
        }
        if matches!(b.front(), Some(RUnit::Insert(_, _))) {
            if let Some(RUnit::Insert(content, attrs)) = b.pop_front() {
                let len = content.len();
                ao.push(RichTextOp::Retain {
                    len,
                    attrs: AttrPatch::new(),
                });
                bo.push(RichTextOp::Insert { content, attrs });
            }
            continue;
        }
        match (a.pop_front(), b.pop_front()) {
            (None, None) => break,
            (Some(RUnit::Retain(patch)), None) => ao.push(RichTextOp::Retain {
                len: 1,
                attrs: patch,
            }),
            (Some(RUnit::Delete), None) => ao.push(RichTextOp::Delete(1)),
            (None, Some(RUnit::Retain(patch))) => bo.push(RichTextOp::Retain {
                len: 1,
                attrs: patch,
            }),
            (None, Some(RUnit::Delete)) => bo.push(RichTextOp::Delete(1)),
            (Some(RUnit::Delete), Some(RUnit::Delete)) => {}
            (Some(RUnit::Delete), Some(RUnit::Retain(_))) => ao.push(RichTextOp::Delete(1)),
            (Some(RUnit::Retain(_)), Some(RUnit::Delete)) => bo.push(RichTextOp::Delete(1)),
            (Some(RUnit::Retain(a)), Some(RUnit::Retain(b))) => {
                let (ap, bp) = transform_attr_patch(&a, &b, tie);
                ao.push(RichTextOp::Retain { len: 1, attrs: ap });
                bo.push(RichTextOp::Retain { len: 1, attrs: bp });
            }
            _ => unreachable!(),
        }
    }
    (RichTextChange::new(ao), RichTextChange::new(bo))
}

fn transform_attr_patch(
    left: &AttrPatch,
    right: &AttrPatch,
    tie: TieBreak,
) -> (AttrPatch, AttrPatch) {
    let mut a = left.to_btree();
    let mut b = right.to_btree();
    let keys: Vec<String> = a
        .keys()
        .filter(|key| b.contains_key(*key))
        .cloned()
        .collect();
    for key in keys {
        if a.get(&key) == b.get(&key) {
            a.remove(&key);
            b.remove(&key);
        } else {
            match tie {
                TieBreak::LeftFirst => {
                    b.remove(&key);
                }
                TieBreak::RightFirst => {
                    a.remove(&key);
                }
            }
        }
    }
    (AttrPatch::from_btree(a), AttrPatch::from_btree(b))
}
