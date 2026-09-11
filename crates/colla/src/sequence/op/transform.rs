use crate::sequence::attrs::AttrPatch;
use crate::sequence::change::{
    Change, ChangeKind, RichTextChange, RichTextOp, TextChange, TextOp, TieBreak,
};
use crate::sequence::error::TransformError;
use crate::sequence::op::reader::{RichTextOpReader, RichTextOpRef, TextOpReader, TextOpRef};

/// Transforms concurrent Changes based on one shared snapshot.
///
/// The returned `(left_prime, right_prime)` satisfies TP1 whenever both
/// transformed execution paths are applicable:
/// `apply(apply(base, left), right_prime) ==
/// apply(apply(base, right), left_prime)`.
///
pub fn transform(
    left: &Change,
    right: &Change,
    tie_break: TieBreak,
) -> Result<(Change, Change), TransformError> {
    transform_change(left, right, tie_break)
}

fn transform_change(
    left: &Change,
    right: &Change,
    tie: TieBreak,
) -> Result<(Change, Change), TransformError> {
    match (left.kind(), right.kind()) {
        (ChangeKind::Noop, _) => Ok((Change::noop(), right.clone())),
        (_, ChangeKind::Noop) => Ok((left.clone(), Change::noop())),
        (ChangeKind::Text(a), ChangeKind::Text(b)) => {
            let (a, b) = transform_text(a, b, tie)?;
            Ok((a.into(), b.into()))
        }
        (ChangeKind::RichText(a), ChangeKind::RichText(b)) => {
            let (a, b) = transform_rich(a, b, tie)?;
            Ok((a.into(), b.into()))
        }
        _ => Err(TransformError::IncompatibleKinds {
            left: left.kind_name(),
            right: right.kind_name(),
        }),
    }
}

fn transform_text(
    left: &TextChange,
    right: &TextChange,
    tie: TieBreak,
) -> Result<(TextChange, TextChange), TransformError> {
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
    let left = TextChange::from_ops(ao).map_err(|_| TransformError::LengthOverflow)?;
    let right = TextChange::from_ops(bo).map_err(|_| TransformError::LengthOverflow)?;
    Ok((left, right))
}

fn transform_rich(
    left: &RichTextChange,
    right: &RichTextChange,
    tie: TieBreak,
) -> Result<(RichTextChange, RichTextChange), TransformError> {
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
            if let Some(RichTextOpRef::Insert { content, .. }) = a.peek() {
                let len = content.len();
                let (content, attrs) = a.take_insert(len).expect("peeked RichText insert");
                ao.push(RichTextOp::Insert { content, attrs });
                bo.push(RichTextOp::Retain {
                    len,
                    attrs: AttrPatch::new(),
                });
            }
            continue;
        }
        if let Some(RichTextOpRef::Insert { content, .. }) = b.peek() {
            let len = content.len();
            let (content, attrs) = b.take_insert(len).expect("peeked RichText insert");
            ao.push(RichTextOp::Retain {
                len,
                attrs: AttrPatch::new(),
            });
            bo.push(RichTextOp::Insert { content, attrs });
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
    let left = RichTextChange::from_ops(ao).map_err(|_| TransformError::LengthOverflow)?;
    let right = RichTextChange::from_ops(bo).map_err(|_| TransformError::LengthOverflow)?;
    Ok((left, right))
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
