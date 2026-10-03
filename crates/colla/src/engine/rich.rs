use super::{Attr, AttrPatch, Attrs, Error, ErrorCode, Result, RichSpan};
use crate::sequence::{attrs as sequence_attrs, change as seq, richtext as rich};
use cocodec::{Decode, Encode};

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// Scalar RichText content and formatting operation.
pub enum RichOp {
    #[cocodec(tag = 0)]
    /// Retains scalar units and optionally changes their formatting.
    Retain {
        /// Length in Unicode scalars; an atomic Embed has length one.
        len: usize,
        /// Formatting changes on retained units; None removes an attribute.
        attrs: AttrPatch,
    },
    #[cocodec(tag = 1)]
    /// Inserts a text span or one atomic embed at the current sequence position.
    Insert(RichSpan),
    #[cocodec(tag = 2)]
    /// Deletes Unicode scalar units from RichText; each embed counts as one.
    Delete(usize),
}
pub(crate) fn attr_to_sequence(value: &Attr) -> Result<sequence_attrs::AttrValue> {
    Ok(match value {
        Attr::Bool(v) => sequence_attrs::AttrValue::Bool(*v),
        Attr::Int(v) => sequence_attrs::AttrValue::Int(*v),
        Attr::Float(v) => sequence_attrs::AttrValue::float(*v)?,
        Attr::String(v) => sequence_attrs::AttrValue::string(v.clone()),
    })
}
fn attr_from_sequence(value: &sequence_attrs::AttrValue) -> Attr {
    match value {
        sequence_attrs::AttrValue::Bool(v) => Attr::Bool(*v),
        sequence_attrs::AttrValue::Int(v) => Attr::Int(*v),
        sequence_attrs::AttrValue::Float(v) => Attr::Float(v.get()),
        sequence_attrs::AttrValue::String(v) => Attr::String(v.to_string()),
    }
}
fn attrs_to_sequence(attrs: &Attrs) -> Result<sequence_attrs::Attrs> {
    Ok(sequence_attrs::Attrs::from_entries(
        attrs
            .iter()
            .map(|(k, v)| Ok((k.clone(), attr_to_sequence(v)?)))
            .collect::<Result<Vec<_>>>()?,
    )?)
}
fn attrs_from_sequence(attrs: &sequence_attrs::Attrs) -> Attrs {
    attrs
        .iter()
        .map(|(k, v)| (k.clone(), attr_from_sequence(v)))
        .collect()
}

fn span_to_sequence(span: &RichSpan) -> Result<rich::RichSpan> {
    Ok(match span {
        RichSpan::Text { text, attrs } => {
            rich::RichSpan::text(text.clone(), attrs_to_sequence(attrs)?)
        }
        RichSpan::Embed { value, attrs } => {
            rich::RichSpan::embed(value.clone(), attrs_to_sequence(attrs)?)
        }
    })
}
fn span_from_sequence(span: &rich::RichSpan) -> Result<RichSpan> {
    Ok(match span.content() {
        rich::RichContent::Text(text) => RichSpan::Text {
            text: text.as_str().into(),
            attrs: attrs_from_sequence(span.attrs()),
        },
        rich::RichContent::Embed(value) => RichSpan::Embed {
            value: value.clone(),
            attrs: attrs_from_sequence(span.attrs()),
        },
    })
}
pub(crate) fn to_sequence_value(spans: &[RichSpan]) -> Result<crate::sequence::value::Value> {
    Ok(crate::sequence::value::Value::rich_text(
        rich::RichText::from_spans(
            spans
                .iter()
                .map(span_to_sequence)
                .collect::<Result<Vec<_>>>()?,
        )?,
    ))
}
pub(crate) fn from_sequence_value(value: &crate::sequence::value::Value) -> Result<Vec<RichSpan>> {
    value
        .as_rich_text()
        .ok_or_else(|| Error::new(ErrorCode::TypeMismatch, "expected RichText"))?
        .iter_spans()
        .map(span_from_sequence)
        .collect()
}
pub(crate) fn to_sequence_change(ops: &[RichOp]) -> Result<seq::Change> {
    let ops = ops
        .iter()
        .map(|op| {
            Ok(match op {
                RichOp::Retain { len, attrs } => seq::RichTextOp::Retain {
                    len: *len,
                    attrs: sequence_attrs::AttrPatch::from_entries(
                        attrs
                            .iter()
                            .map(|(k, v)| {
                                Ok((
                                    k.clone(),
                                    match v {
                                        Some(v) => {
                                            sequence_attrs::AttrChange::Set(attr_to_sequence(v)?)
                                        }
                                        None => sequence_attrs::AttrChange::Remove,
                                    },
                                ))
                            })
                            .collect::<Result<Vec<_>>>()?,
                    )?,
                },
                RichOp::Insert(span) => {
                    let span = span_to_sequence(span)?;
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
pub(crate) fn from_sequence_change(change: &seq::Change) -> Result<Vec<RichOp>> {
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
                                    sequence_attrs::AttrChange::Set(v) => {
                                        Some(attr_from_sequence(v))
                                    }
                                    sequence_attrs::AttrChange::Remove => None,
                                },
                            )
                        })
                        .collect(),
                },
                seq::RichTextOp::Insert { content, attrs } => RichOp::Insert(match content {
                    rich::RichContent::Text(text) => RichSpan::Text {
                        text: text.as_str().into(),
                        attrs: attrs_from_sequence(attrs),
                    },
                    rich::RichContent::Embed(value) => RichSpan::Embed {
                        value: value.clone(),
                        attrs: attrs_from_sequence(attrs),
                    },
                }),
                seq::RichTextOp::Delete(len) => RichOp::Delete(*len),
            })
        })
        .collect()
}
pub(crate) fn normalized(ops: &[RichOp]) -> Result<Vec<RichOp>> {
    from_sequence_change(&to_sequence_change(ops)?)
}
