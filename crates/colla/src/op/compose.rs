use std::collections::BTreeMap;

use crate::attrs::AttrPatch;
use crate::change::{
    Change, ChangeKind, IntChange, ListChange, ListOp, MapChange, MapEntryChange, RichTextChange,
    RichTextOp, TextChange, TextOp,
};
use crate::error::ComposeError;
use crate::op::reader::{
    text_prefix, ListOpReader, ListOpRef, RichTextOpReader, RichTextOpRef, TextOpReader, TextOpRef,
};

impl Change {
    /// Sequentially composes `self` followed by `next`.
    pub fn compose(&self, next: &Change) -> Result<Change, ComposeError> {
        compose_change(self, next)
    }
}

fn compose_change(left: &Change, right: &Change) -> Result<Change, ComposeError> {
    match (left.kind(), right.kind()) {
        (ChangeKind::Noop, _) => Ok(right.clone()),
        (_, ChangeKind::Noop) => Ok(left.clone()),
        (_, ChangeKind::Replace(value)) => Ok(Change::replace(value.clone())),
        (ChangeKind::Replace(value), _) => Ok(Change::replace(right.apply_to(value)?)),
        (ChangeKind::Int(IntChange::Add(a)), ChangeKind::Int(IntChange::Add(b))) => {
            let sum = a.checked_add(*b).ok_or_else(|| {
                ComposeError::Apply(crate::error::ApplyError::IntegerOverflow {
                    path: crate::Path::new(),
                })
            })?;
            Ok(Change::int_add(sum))
        }
        (ChangeKind::Map(a), ChangeKind::Map(b)) => compose_map(a, b),
        (ChangeKind::List(a), ChangeKind::List(b)) => compose_list(a, b),
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

fn compose_map(left: &MapChange, right: &MapChange) -> Result<Change, ComposeError> {
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
                Some(MapEntryChange::Insert(change.apply_to(value)?))
            }
            (Some(MapEntryChange::Insert(_)), Some(MapEntryChange::Delete)) => None,
            (Some(MapEntryChange::Delete), Some(MapEntryChange::Insert(value))) => {
                Some(MapEntryChange::Modify(Change::replace(value.clone())))
            }
            (Some(MapEntryChange::Modify(a)), Some(MapEntryChange::Modify(b))) => {
                let child = compose_change(a, b)?;
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

fn compose_text(left: &TextChange, right: &TextChange) -> TextChange {
    let mut a = TextOpReader::new(left.ops());
    let mut b = TextOpReader::new(right.ops());
    let mut out = Vec::new();
    loop {
        if let Some(TextOpRef::Insert { text, len }) = b.peek() {
            out.push(TextOp::Insert(text.to_owned()));
            b.consume(len);
            continue;
        }
        if let Some(TextOpRef::Delete(len)) = a.peek() {
            out.push(TextOp::Delete(len));
            a.consume(len);
            continue;
        }
        match (a.peek(), b.peek()) {
            (None, None) => break,
            (None, Some(TextOpRef::Retain(len))) => {
                out.push(TextOp::Retain(len));
                b.consume(len);
            }
            (None, Some(TextOpRef::Delete(len))) => {
                out.push(TextOp::Delete(len));
                b.consume(len);
            }
            (Some(TextOpRef::Retain(len)), None) => {
                out.push(TextOp::Retain(len));
                a.consume(len);
            }
            (Some(TextOpRef::Insert { text, len }), None) => {
                out.push(TextOp::Insert(text.to_owned()));
                a.consume(len);
            }
            (
                Some(TextOpRef::Insert {
                    text,
                    len: left_len,
                }),
                Some(TextOpRef::Retain(right_len)),
            ) => {
                let len = left_len.min(right_len);
                out.push(TextOp::Insert(text_prefix(text, len).to_owned()));
                a.consume(len);
                b.consume(len);
            }
            (Some(TextOpRef::Insert { len: left_len, .. }), Some(TextOpRef::Delete(right_len))) => {
                let len = left_len.min(right_len);
                a.consume(len);
                b.consume(len);
            }
            (Some(TextOpRef::Retain(left_len)), Some(TextOpRef::Retain(right_len))) => {
                let len = left_len.min(right_len);
                out.push(TextOp::Retain(len));
                a.consume(len);
                b.consume(len);
            }
            (Some(TextOpRef::Retain(left_len)), Some(TextOpRef::Delete(right_len))) => {
                let len = left_len.min(right_len);
                out.push(TextOp::Delete(len));
                a.consume(len);
                b.consume(len);
            }
            _ => unreachable!(),
        }
    }
    TextChange::new(out)
}

fn compose_list(left: &ListChange, right: &ListChange) -> Result<Change, ComposeError> {
    let mut a = ListOpReader::new(left.ops());
    let mut b = ListOpReader::new(right.ops());
    let mut out = Vec::new();
    loop {
        if let Some(ListOpRef::Insert(values)) = b.peek() {
            let len = values.len();
            out.push(ListOp::Insert(values.to_vec()));
            b.consume(len);
            continue;
        }
        if let Some(ListOpRef::Delete(len)) = a.peek() {
            out.push(ListOp::Delete(len));
            a.consume(len);
            continue;
        }
        match (a.peek(), b.peek()) {
            (None, None) => break,
            (None, Some(ListOpRef::Retain(len))) => {
                out.push(ListOp::Retain(len));
                b.consume(len);
            }
            (None, Some(ListOpRef::Delete(len))) => {
                out.push(ListOp::Delete(len));
                b.consume(len);
            }
            (None, Some(ListOpRef::Modify(change))) => {
                out.push(ListOp::Modify(change.clone()));
                b.consume(1);
            }
            (Some(ListOpRef::Retain(len)), None) => {
                out.push(ListOp::Retain(len));
                a.consume(len);
            }
            (Some(ListOpRef::Insert(values)), None) => {
                let len = values.len();
                out.push(ListOp::Insert(values.to_vec()));
                a.consume(len);
            }
            (Some(ListOpRef::Modify(change)), None) => {
                out.push(ListOp::Modify(change.clone()));
                a.consume(1);
            }
            (Some(ListOpRef::Insert(values)), Some(ListOpRef::Retain(right_len))) => {
                let len = values.len().min(right_len);
                out.push(ListOp::Insert(values[..len].to_vec()));
                a.consume(len);
                b.consume(len);
            }
            (Some(ListOpRef::Insert(values)), Some(ListOpRef::Delete(right_len))) => {
                let len = values.len().min(right_len);
                a.consume(len);
                b.consume(len);
            }
            (Some(ListOpRef::Insert(values)), Some(ListOpRef::Modify(change))) => {
                out.push(ListOp::Insert(vec![change.apply_to(&values[0])?]));
                a.consume(1);
                b.consume(1);
            }
            (Some(ListOpRef::Retain(left_len)), Some(ListOpRef::Retain(right_len))) => {
                let len = left_len.min(right_len);
                out.push(ListOp::Retain(len));
                a.consume(len);
                b.consume(len);
            }
            (Some(ListOpRef::Retain(left_len)), Some(ListOpRef::Delete(right_len))) => {
                let len = left_len.min(right_len);
                out.push(ListOp::Delete(len));
                a.consume(len);
                b.consume(len);
            }
            (Some(ListOpRef::Modify(_)), Some(ListOpRef::Delete(_))) => {
                out.push(ListOp::Delete(1));
                a.consume(1);
                b.consume(1);
            }
            (Some(ListOpRef::Retain(_)), Some(ListOpRef::Modify(change))) => {
                out.push(ListOp::Modify(change.clone()));
                a.consume(1);
                b.consume(1);
            }
            (Some(ListOpRef::Modify(change)), Some(ListOpRef::Retain(_))) => {
                out.push(ListOp::Modify(change.clone()));
                a.consume(1);
                b.consume(1);
            }
            (Some(ListOpRef::Modify(left)), Some(ListOpRef::Modify(right))) => {
                out.push(ListOp::Modify(compose_change(left, right)?));
                a.consume(1);
                b.consume(1);
            }
            _ => unreachable!(),
        }
    }
    Ok(Change::list(ListChange::new(out)))
}

fn compose_rich(left: &RichTextChange, right: &RichTextChange) -> RichTextChange {
    let mut a = RichTextOpReader::new(left.ops());
    let mut b = RichTextOpReader::new(right.ops());
    let mut out = Vec::new();
    loop {
        if let Some(RichTextOpRef::Insert { content, .. }) = b.peek() {
            let len = content.len();
            let (content, taken_attrs) = b.take_insert(len).expect("peeked RichText insert");
            out.push(RichTextOp::Insert {
                content,
                attrs: taken_attrs,
            });
            continue;
        }
        if let Some(RichTextOpRef::Delete(len)) = a.peek() {
            out.push(RichTextOp::Delete(len));
            a.consume(len);
            continue;
        }
        match (a.peek(), b.peek()) {
            (None, None) => break,
            (None, Some(RichTextOpRef::Retain { len, attrs })) => {
                out.push(RichTextOp::Retain {
                    len,
                    attrs: attrs.clone(),
                });
                b.consume(len);
            }
            (None, Some(RichTextOpRef::Delete(len))) => {
                out.push(RichTextOp::Delete(len));
                b.consume(len);
            }
            (Some(RichTextOpRef::Retain { len, attrs }), None) => {
                out.push(RichTextOp::Retain {
                    len,
                    attrs: attrs.clone(),
                });
                a.consume(len);
            }
            (Some(RichTextOpRef::Insert { content, .. }), None) => {
                let len = content.len();
                let (content, taken_attrs) = a.take_insert(len).expect("peeked RichText insert");
                out.push(RichTextOp::Insert {
                    content,
                    attrs: taken_attrs,
                });
            }
            (
                Some(RichTextOpRef::Insert { content }),
                Some(RichTextOpRef::Retain {
                    len: right_len,
                    attrs: patch,
                }),
            ) => {
                let len = content.len().min(right_len);
                let (content, attrs) = a.take_insert(len).expect("peeked RichText insert");
                out.push(RichTextOp::Insert {
                    content,
                    attrs: attrs.apply_patch(patch),
                });
                b.consume(len);
            }
            (
                Some(RichTextOpRef::Insert { content, .. }),
                Some(RichTextOpRef::Delete(right_len)),
            ) => {
                let len = content.len().min(right_len);
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
                out.push(RichTextOp::Retain {
                    len,
                    attrs: compose_attr_patch(left_attrs, right_attrs),
                });
                a.consume(len);
                b.consume(len);
            }
            (
                Some(RichTextOpRef::Retain { len: left_len, .. }),
                Some(RichTextOpRef::Delete(right_len)),
            ) => {
                let len = left_len.min(right_len);
                out.push(RichTextOp::Delete(len));
                a.consume(len);
                b.consume(len);
            }
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
