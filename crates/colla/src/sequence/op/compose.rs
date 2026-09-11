use crate::sequence::attrs::AttrPatch;
use crate::sequence::change::{Change, ChangeKind, RichTextChange, RichTextOp, TextChange, TextOp};
use crate::sequence::error::ComposeError;
use crate::sequence::op::reader::{
    text_prefix, RichTextOpReader, RichTextOpRef, TextOpReader, TextOpRef,
};

impl Change {
    /// Sequentially composes `self` followed by `next`.
    ///
    /// This is the inherent equivalent of [`crate::sequence::op::compose`].
    pub fn compose(&self, next: &Change) -> Result<Change, ComposeError> {
        compose_change(self, next)
    }
}

fn compose_change(left: &Change, right: &Change) -> Result<Change, ComposeError> {
    match (left.kind(), right.kind()) {
        (ChangeKind::Noop, _) => Ok(right.clone()),
        (_, ChangeKind::Noop) => Ok(left.clone()),
        (ChangeKind::Text(a), ChangeKind::Text(b)) => Ok(compose_text(a, b)?.into()),
        (ChangeKind::RichText(a), ChangeKind::RichText(b)) => Ok(compose_rich(a, b)?.into()),
        _ => Err(ComposeError::IncompatibleKinds {
            left: left.kind_name(),
            right: right.kind_name(),
        }),
    }
}

fn compose_text(left: &TextChange, right: &TextChange) -> Result<TextChange, ComposeError> {
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
    TextChange::from_ops(out).map_err(|_| ComposeError::LengthOverflow)
}

fn compose_rich(
    left: &RichTextChange,
    right: &RichTextChange,
) -> Result<RichTextChange, ComposeError> {
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
    RichTextChange::from_ops(out).map_err(|_| ComposeError::LengthOverflow)
}

pub(crate) fn compose_attr_patch(left: &AttrPatch, right: &AttrPatch) -> AttrPatch {
    let mut out = left.to_btree();
    for (key, value) in right.iter() {
        out.insert(key.clone(), value.clone());
    }
    AttrPatch::from_btree(out)
}
