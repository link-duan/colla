use std::collections::BTreeMap;

use crate::attrs::AttrPatch;
use crate::change::{
    Change, ChangeKind, ListChange, ListOp, MapChange, MapEntryChange, RichTextChange, RichTextOp,
    TextChange, TextOp, TieBreak,
};
use crate::error::TransformError;
use crate::limits::Limits;
use crate::op::reader::{
    ListOpReader, ListOpRef, RichTextOpReader, RichTextOpRef, TextOpReader, TextOpRef,
};

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

fn transform_text(
    left: &TextChange,
    right: &TextChange,
    tie: TieBreak,
) -> (TextChange, TextChange) {
    let mut a = TextOpReader::new(left.ops());
    let mut b = TextOpReader::new(right.ops());
    let mut ao = Vec::new();
    let mut bo = Vec::new();
    loop {
        let both_insert = matches!(a.peek(), Some(TextOpRef::Insert { .. }))
            && matches!(b.peek(), Some(TextOpRef::Insert { .. }));
        if matches!(a.peek(), Some(TextOpRef::Insert { .. }))
            && (!both_insert || tie == TieBreak::LeftFirst)
        {
            if let Some(TextOpRef::Insert { text, len }) = a.peek() {
                ao.push(TextOp::Insert(text.to_owned()));
                bo.push(TextOp::Retain(len));
                a.consume(len);
            }
            continue;
        }
        if let Some(TextOpRef::Insert { text, len }) = b.peek() {
            ao.push(TextOp::Retain(len));
            bo.push(TextOp::Insert(text.to_owned()));
            b.consume(len);
            continue;
        }
        match (a.peek(), b.peek()) {
            (None, None) => break,
            (Some(TextOpRef::Retain(len)), None) => {
                ao.push(TextOp::Retain(len));
                a.consume(len);
            }
            (Some(TextOpRef::Delete(len)), None) => {
                ao.push(TextOp::Delete(len));
                a.consume(len);
            }
            (None, Some(TextOpRef::Retain(len))) => {
                bo.push(TextOp::Retain(len));
                b.consume(len);
            }
            (None, Some(TextOpRef::Delete(len))) => {
                bo.push(TextOp::Delete(len));
                b.consume(len);
            }
            (Some(TextOpRef::Delete(left_len)), Some(TextOpRef::Delete(right_len))) => {
                let len = left_len.min(right_len);
                a.consume(len);
                b.consume(len);
            }
            (Some(TextOpRef::Delete(left_len)), Some(TextOpRef::Retain(right_len))) => {
                let len = left_len.min(right_len);
                ao.push(TextOp::Delete(len));
                a.consume(len);
                b.consume(len);
            }
            (Some(TextOpRef::Retain(left_len)), Some(TextOpRef::Delete(right_len))) => {
                let len = left_len.min(right_len);
                bo.push(TextOp::Delete(len));
                a.consume(len);
                b.consume(len);
            }
            (Some(TextOpRef::Retain(left_len)), Some(TextOpRef::Retain(right_len))) => {
                let len = left_len.min(right_len);
                ao.push(TextOp::Retain(len));
                bo.push(TextOp::Retain(len));
                a.consume(len);
                b.consume(len);
            }
            _ => unreachable!(),
        }
    }
    (TextChange::new(ao), TextChange::new(bo))
}

fn transform_list(
    left: &ListChange,
    right: &ListChange,
    tie: TieBreak,
    limits: &Limits,
) -> Result<(Change, Change), TransformError> {
    let mut a = ListOpReader::new(left.ops());
    let mut b = ListOpReader::new(right.ops());
    let mut ao = Vec::new();
    let mut bo = Vec::new();
    loop {
        let both_insert = matches!(a.peek(), Some(ListOpRef::Insert(_)))
            && matches!(b.peek(), Some(ListOpRef::Insert(_)));
        if matches!(a.peek(), Some(ListOpRef::Insert(_)))
            && (!both_insert || tie == TieBreak::LeftFirst)
        {
            if let Some(ListOpRef::Insert(values)) = a.peek() {
                let len = values.len();
                ao.push(ListOp::Insert(values.to_vec()));
                bo.push(ListOp::Retain(len));
                a.consume(len);
            }
            continue;
        }
        if let Some(ListOpRef::Insert(values)) = b.peek() {
            let len = values.len();
            ao.push(ListOp::Retain(len));
            bo.push(ListOp::Insert(values.to_vec()));
            b.consume(len);
            continue;
        }
        match (a.peek(), b.peek()) {
            (None, None) => break,
            (Some(ListOpRef::Retain(len)), None) => {
                ao.push(ListOp::Retain(len));
                a.consume(len);
            }
            (Some(ListOpRef::Delete(len)), None) => {
                ao.push(ListOp::Delete(len));
                a.consume(len);
            }
            (Some(ListOpRef::Modify(change)), None) => {
                ao.push(ListOp::Modify(change.clone()));
                a.consume(1);
            }
            (None, Some(ListOpRef::Retain(len))) => {
                bo.push(ListOp::Retain(len));
                b.consume(len);
            }
            (None, Some(ListOpRef::Delete(len))) => {
                bo.push(ListOp::Delete(len));
                b.consume(len);
            }
            (None, Some(ListOpRef::Modify(change))) => {
                bo.push(ListOp::Modify(change.clone()));
                b.consume(1);
            }
            (Some(ListOpRef::Delete(left_len)), Some(ListOpRef::Delete(right_len))) => {
                let len = left_len.min(right_len);
                a.consume(len);
                b.consume(len);
            }
            (Some(ListOpRef::Delete(left_len)), Some(ListOpRef::Retain(right_len))) => {
                let len = left_len.min(right_len);
                ao.push(ListOp::Delete(len));
                a.consume(len);
                b.consume(len);
            }
            (Some(ListOpRef::Delete(_)), Some(ListOpRef::Modify(_))) => {
                ao.push(ListOp::Delete(1));
                a.consume(1);
                b.consume(1);
            }
            (Some(ListOpRef::Retain(left_len)), Some(ListOpRef::Delete(right_len))) => {
                let len = left_len.min(right_len);
                bo.push(ListOp::Delete(len));
                a.consume(len);
                b.consume(len);
            }
            (Some(ListOpRef::Modify(_)), Some(ListOpRef::Delete(_))) => {
                bo.push(ListOp::Delete(1));
                a.consume(1);
                b.consume(1);
            }
            (Some(ListOpRef::Retain(left_len)), Some(ListOpRef::Retain(right_len))) => {
                let len = left_len.min(right_len);
                ao.push(ListOp::Retain(len));
                bo.push(ListOp::Retain(len));
                a.consume(len);
                b.consume(len);
            }
            (Some(ListOpRef::Modify(change)), Some(ListOpRef::Retain(_))) => {
                ao.push(ListOp::Modify(change.clone()));
                bo.push(ListOp::Retain(1));
                a.consume(1);
                b.consume(1);
            }
            (Some(ListOpRef::Retain(_)), Some(ListOpRef::Modify(change))) => {
                ao.push(ListOp::Retain(1));
                bo.push(ListOp::Modify(change.clone()));
                a.consume(1);
                b.consume(1);
            }
            (Some(ListOpRef::Modify(left)), Some(ListOpRef::Modify(right))) => {
                let (ap, bp) = transform_change(left, right, tie, limits)?;
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
                a.consume(1);
                b.consume(1);
            }
            _ => unreachable!(),
        }
    }
    Ok((
        Change::list(ListChange::new(ao)),
        Change::list(ListChange::new(bo)),
    ))
}

fn transform_rich(
    left: &RichTextChange,
    right: &RichTextChange,
    tie: TieBreak,
) -> (RichTextChange, RichTextChange) {
    let mut a = RichTextOpReader::new(left.ops());
    let mut b = RichTextOpReader::new(right.ops());
    let mut ao = Vec::new();
    let mut bo = Vec::new();
    loop {
        let both_insert = matches!(a.peek(), Some(RichTextOpRef::Insert { .. }))
            && matches!(b.peek(), Some(RichTextOpRef::Insert { .. }));
        if matches!(a.peek(), Some(RichTextOpRef::Insert { .. }))
            && (!both_insert || tie == TieBreak::LeftFirst)
        {
            if let Some(RichTextOpRef::Insert { content, attrs }) = a.peek() {
                let len = content.len();
                ao.push(RichTextOp::Insert {
                    content: content.prefix(len),
                    attrs: attrs.clone(),
                });
                bo.push(RichTextOp::Retain {
                    len,
                    attrs: AttrPatch::new(),
                });
                a.consume(len);
            }
            continue;
        }
        if let Some(RichTextOpRef::Insert { content, attrs }) = b.peek() {
            let len = content.len();
            ao.push(RichTextOp::Retain {
                len,
                attrs: AttrPatch::new(),
            });
            bo.push(RichTextOp::Insert {
                content: content.prefix(len),
                attrs: attrs.clone(),
            });
            b.consume(len);
            continue;
        }
        match (a.peek(), b.peek()) {
            (None, None) => break,
            (Some(RichTextOpRef::Retain { len, attrs }), None) => {
                ao.push(RichTextOp::Retain {
                    len,
                    attrs: attrs.clone(),
                });
                a.consume(len);
            }
            (Some(RichTextOpRef::Delete(len)), None) => {
                ao.push(RichTextOp::Delete(len));
                a.consume(len);
            }
            (None, Some(RichTextOpRef::Retain { len, attrs })) => {
                bo.push(RichTextOp::Retain {
                    len,
                    attrs: attrs.clone(),
                });
                b.consume(len);
            }
            (None, Some(RichTextOpRef::Delete(len))) => {
                bo.push(RichTextOp::Delete(len));
                b.consume(len);
            }
            (Some(RichTextOpRef::Delete(left_len)), Some(RichTextOpRef::Delete(right_len))) => {
                let len = left_len.min(right_len);
                a.consume(len);
                b.consume(len);
            }
            (
                Some(RichTextOpRef::Delete(left_len)),
                Some(RichTextOpRef::Retain { len: right_len, .. }),
            ) => {
                let len = left_len.min(right_len);
                ao.push(RichTextOp::Delete(len));
                a.consume(len);
                b.consume(len);
            }
            (
                Some(RichTextOpRef::Retain { len: left_len, .. }),
                Some(RichTextOpRef::Delete(right_len)),
            ) => {
                let len = left_len.min(right_len);
                bo.push(RichTextOp::Delete(len));
                a.consume(len);
                b.consume(len);
            }
            (
                Some(RichTextOpRef::Retain {
                    len: left_len,
                    attrs: left_attrs,
                }),
                Some(RichTextOpRef::Retain {
                    len: right_len,
                    attrs: right_attrs,
                }),
            ) => {
                let len = left_len.min(right_len);
                let (ap, bp) = transform_attr_patch(left_attrs, right_attrs, tie);
                ao.push(RichTextOp::Retain { len, attrs: ap });
                bo.push(RichTextOp::Retain { len, attrs: bp });
                a.consume(len);
                b.consume(len);
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
