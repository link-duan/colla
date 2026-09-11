use super::{Attr, AttrPatch, Attrs, Error, ErrorCode, Result, RichSpan};
use crate::sequence::{attrs as old, change as seq, richtext as rich};
use cocodec::{Decode, Encode};

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// Scalar RichText content and formatting operation.
pub enum RichOp {
    #[cocodec(tag = 0)]
    /// Retains scalar units and optionally changes their formatting.
    Retain {
        /// Length in Unicode scalars; an atomic Embed has length one.
        len: usize,
        /// Atomic formatting attributes or their explicit patch.
        attrs: AttrPatch,
    },
    #[cocodec(tag = 1)]
    /// Inserts owning content at a sequence or container position.
    Insert(RichSpan),
    #[cocodec(tag = 2)]
    /// Deletes existing owning content or scalar sequence units.
    Delete(usize),
}
pub(crate) fn attr_to_old(value: &Attr) -> Result<old::AttrValue> {
    Ok(match value {
        Attr::Bool(v) => old::AttrValue::Bool(*v),
        Attr::Int(v) => old::AttrValue::Int(*v),
        Attr::Float(v) => old::AttrValue::float(*v)?,
        Attr::String(v) => old::AttrValue::string(v.clone()),
    })
}
fn attr_from_old(value: &old::AttrValue) -> Attr {
    match value {
        old::AttrValue::Bool(v) => Attr::Bool(*v),
        old::AttrValue::Int(v) => Attr::Int(*v),
        old::AttrValue::Float(v) => Attr::Float(v.get()),
        old::AttrValue::String(v) => Attr::String(v.to_string()),
    }
}
fn attrs_to_old(attrs: &Attrs) -> Result<old::Attrs> {
    Ok(old::Attrs::from_entries(
        attrs
            .iter()
            .map(|(k, v)| Ok((k.clone(), attr_to_old(v)?)))
            .collect::<Result<Vec<_>>>()?,
    )?)
}
fn attrs_from_old(attrs: &old::Attrs) -> Attrs {
    attrs
        .iter()
        .map(|(k, v)| (k.clone(), attr_from_old(v)))
        .collect()
}

fn span_to_old(span: &RichSpan) -> Result<rich::RichSpan> {
    Ok(match span {
        RichSpan::Text { text, attrs } => rich::RichSpan::text(text.clone(), attrs_to_old(attrs)?),
        RichSpan::Embed { value, attrs } => {
            rich::RichSpan::embed(value.clone(), attrs_to_old(attrs)?)
        }
    })
}
fn span_from_old(span: &rich::RichSpan) -> Result<RichSpan> {
    Ok(match span.content() {
        rich::RichContent::Text(text) => RichSpan::Text {
            text: text.as_str().into(),
            attrs: attrs_from_old(span.attrs()),
        },
        rich::RichContent::Embed(value) => RichSpan::Embed {
            value: value.clone(),
            attrs: attrs_from_old(span.attrs()),
        },
    })
}
pub(crate) fn value_to_old(spans: &[RichSpan]) -> Result<crate::sequence::value::Value> {
    Ok(crate::sequence::value::Value::rich_text(
        rich::RichText::from_spans(spans.iter().map(span_to_old).collect::<Result<Vec<_>>>()?)?,
    ))
}
pub(crate) fn value_from_old(value: &crate::sequence::value::Value) -> Result<Vec<RichSpan>> {
    value
        .as_rich_text()
        .ok_or_else(|| Error::new(ErrorCode::TypeMismatch, "expected RichText"))?
        .iter_spans()
        .map(span_from_old)
        .collect()
}
pub(crate) fn to_old(ops: &[RichOp]) -> Result<seq::Change> {
    let ops = ops
        .iter()
        .map(|op| {
            Ok(match op {
                RichOp::Retain { len, attrs } => seq::RichTextOp::Retain {
                    len: *len,
                    attrs: old::AttrPatch::from_entries(
                        attrs
                            .iter()
                            .map(|(k, v)| {
                                Ok((
                                    k.clone(),
                                    match v {
                                        Some(v) => old::AttrChange::Set(attr_to_old(v)?),
                                        None => old::AttrChange::Remove,
                                    },
                                ))
                            })
                            .collect::<Result<Vec<_>>>()?,
                    )?,
                },
                RichOp::Insert(span) => {
                    let span = span_to_old(span)?;
                    seq::RichTextOp::Insert {
                        content: span.content().clone(),
                        attrs: span.attrs().clone(),
                    }
                }
                RichOp::Delete(len) => seq::RichTextOp::Delete(*len),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(seq::RichTextChange::from_ops(ops)?.into())
}
pub(crate) fn from_old(change: &seq::Change) -> Result<Vec<RichOp>> {
    let Some(change) = change.as_rich_text() else {
        return Ok(Vec::new());
    };
    change
        .ops()
        .iter()
        .map(|op| {
            Ok(match op {
                seq::RichTextOp::Retain { len, attrs } => RichOp::Retain {
                    len: *len,
                    attrs: attrs
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.clone(),
                                match v {
                                    old::AttrChange::Set(v) => Some(attr_from_old(v)),
                                    old::AttrChange::Remove => None,
                                },
                            )
                        })
                        .collect(),
                },
                seq::RichTextOp::Insert { content, attrs } => RichOp::Insert(match content {
                    rich::RichContent::Text(text) => RichSpan::Text {
                        text: text.as_str().into(),
                        attrs: attrs_from_old(attrs),
                    },
                    rich::RichContent::Embed(value) => RichSpan::Embed {
                        value: value.clone(),
                        attrs: attrs_from_old(attrs),
                    },
                }),
                seq::RichTextOp::Delete(len) => RichOp::Delete(*len),
            })
        })
        .collect()
}
pub(crate) fn normalized(ops: &[RichOp]) -> Result<Vec<RichOp>> {
    from_old(&to_old(ops)?)
}
