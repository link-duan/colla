//! Canonical scalar sequence operations. Structural edits live in the identity engine.
use crate::sequence::attrs::{AttrPatch, Attrs};
use crate::sequence::error::ValueError;
use crate::sequence::richtext::RichContent;
use std::sync::Arc;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TieBreak {
    LeftFirst,
    RightFirst,
}
/// An operation in a Text Change stream.
///
/// Retain and Delete lengths count Unicode scalar values.
#[derive(Debug, Clone, PartialEq, Eq, cocodec::Encode, cocodec::Decode)]
pub enum TextOp {
    /// Retains this many Unicode scalars unchanged.
    #[cocodec(tag = 0)]
    Retain(usize),
    /// Inserts UTF-8 text at the current position.
    #[cocodec(tag = 1)]
    Insert(String),
    /// Deletes this many Unicode scalars from the base Text.
    #[cocodec(tag = 2)]
    Delete(usize),
}

/// A canonical Text operation stream.
#[derive(Debug, Clone, Default, PartialEq, Eq, cocodec::Encode, cocodec::Decode)]
#[cocodec(transparent)]
pub struct TextChange(Arc<Vec<TextOp>>);

impl TextChange {
    /// Creates and normalizes a Text operation stream.
    ///
    /// Zero operations and empty inserts are removed, compatible operations
    /// are merged, and trailing retains are omitted.
    ///
    pub fn from_ops<I>(ops: I) -> crate::engine::Result<Self>
    where
        I: IntoIterator<Item = TextOp>,
    {
        let ops = normalize_text(ops)?;
        validate_text_lengths(&ops)?;
        Ok(Self(Arc::new(ops)))
    }
    /// Returns the canonical operations.
    pub fn ops(&self) -> &[TextOp] {
        &self.0
    }
    /// Returns whether the canonical stream contains no operations.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A rich-text retain keeps `len` sequence units and applies `attrs` to each
/// retained character or embed. An empty patch is a plain retain.
#[derive(Debug, Clone, PartialEq, Eq, cocodec::Encode, cocodec::Decode)]
pub enum RichTextOp {
    /// Retains content and applies `attrs` to every retained scalar or embed.
    #[cocodec(tag = 0)]
    Retain {
        /// Logical length in Unicode scalars and atomic embeds.
        len: usize,
        /// Attribute changes; an empty patch is a plain retain.
        attrs: AttrPatch,
    },
    /// Inserts text or one atomic embed with the supplied attributes.
    #[cocodec(tag = 1)]
    Insert {
        /// Text or embed content.
        content: RichContent,
        /// Attributes assigned to the inserted content.
        attrs: Attrs,
    },
    /// Deletes this many Unicode scalars or atomic embeds.
    #[cocodec(tag = 2)]
    Delete(usize),
}

/// A canonical RichText operation stream.
#[derive(Debug, Clone, Default, PartialEq, Eq, cocodec::Encode, cocodec::Decode)]
#[cocodec(transparent)]
pub struct RichTextChange(Arc<Vec<RichTextOp>>);

impl RichTextChange {
    /// Creates and normalizes a RichText operation stream.
    ///
    /// Compatible text inserts and identical retain patches are merged, empty
    /// operations are removed, and trailing plain retains are omitted.
    pub fn from_ops<I>(ops: I) -> Result<Self, ValueError>
    where
        I: IntoIterator<Item = RichTextOp>,
    {
        let ops = normalize_rich(ops)?;
        validate_rich_text_lengths(&ops)?;
        Ok(Self(Arc::new(ops)))
    }
    /// Returns the canonical operations.
    pub fn ops(&self) -> &[RichTextOp] {
        &self.0
    }
    /// Returns whether the canonical stream contains no operations.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    Noop,
    Text(TextChange),
    RichText(RichTextChange),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change(Arc<ChangeKind>);
impl Default for Change {
    fn default() -> Self {
        Self::noop()
    }
}
impl Change {
    pub fn noop() -> Self {
        Self(Arc::new(ChangeKind::Noop))
    }
    pub fn kind(&self) -> &ChangeKind {
        &self.0
    }
    pub fn kind_name(&self) -> &'static str {
        match self.kind() {
            ChangeKind::Noop => "noop",
            ChangeKind::Text(_) => "text",
            ChangeKind::RichText(_) => "richtext",
        }
    }
    pub fn as_text(&self) -> Option<&TextChange> {
        if let ChangeKind::Text(v) = self.kind() {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_rich_text(&self) -> Option<&RichTextChange> {
        if let ChangeKind::RichText(v) = self.kind() {
            Some(v)
        } else {
            None
        }
    }
}
impl From<TextChange> for Change {
    fn from(v: TextChange) -> Self {
        if v.is_empty() {
            Self::noop()
        } else {
            Self(Arc::new(ChangeKind::Text(v)))
        }
    }
}
impl From<RichTextChange> for Change {
    fn from(v: RichTextChange) -> Self {
        if v.is_empty() {
            Self::noop()
        } else {
            Self(Arc::new(ChangeKind::RichText(v)))
        }
    }
}
#[derive(Default)]
struct ChangeLengths {
    input: usize,
    output: usize,
}

impl ChangeLengths {
    fn retain(&mut self, len: usize) -> Result<(), ValueError> {
        self.input = checked_sequence_add(self.input, len)?;
        self.output = checked_sequence_add(self.output, len)?;
        Ok(())
    }

    fn insert(&mut self, len: usize) -> Result<(), ValueError> {
        self.output = checked_sequence_add(self.output, len)?;
        Ok(())
    }

    fn delete(&mut self, len: usize) -> Result<(), ValueError> {
        self.input = checked_sequence_add(self.input, len)?;
        Ok(())
    }
}

fn checked_sequence_add(left: usize, right: usize) -> Result<usize, ValueError> {
    left.checked_add(right).ok_or(ValueError::LengthOverflow)
}

fn validate_text_lengths(ops: &[TextOp]) -> Result<(), ValueError> {
    let mut lengths = ChangeLengths::default();
    for op in ops {
        match op {
            TextOp::Retain(len) => lengths.retain(*len)?,
            TextOp::Insert(value) => lengths.insert(value.chars().count())?,
            TextOp::Delete(len) => lengths.delete(*len)?,
        }
    }
    Ok(())
}

fn validate_rich_text_lengths(ops: &[RichTextOp]) -> Result<(), ValueError> {
    let mut lengths = ChangeLengths::default();
    for op in ops {
        match op {
            RichTextOp::Retain { len, .. } => lengths.retain(*len)?,
            RichTextOp::Insert { content, .. } => lengths.insert(content.len())?,
            RichTextOp::Delete(len) => lengths.delete(*len)?,
        }
    }
    Ok(())
}

fn normalize_text<I>(ops: I) -> Result<Vec<TextOp>, ValueError>
where
    I: IntoIterator<Item = TextOp>,
{
    let mut out = Vec::new();
    for op in ops {
        let op = match op {
            TextOp::Retain(0) | TextOp::Delete(0) => continue,
            TextOp::Insert(value) if value.is_empty() => continue,
            other => other,
        };
        push_text(&mut out, op)?;
    }
    while matches!(out.last(), Some(TextOp::Retain(_))) {
        out.pop();
    }
    Ok(out)
}

fn push_text(out: &mut Vec<TextOp>, op: TextOp) -> Result<(), ValueError> {
    if matches!(op, TextOp::Insert(_)) && matches!(out.last(), Some(TextOp::Delete(_))) {
        let delete = out.pop().unwrap();
        push_text(out, op)?;
        push_text(out, delete)?;
        return Ok(());
    }
    match (out.last_mut(), op) {
        (Some(TextOp::Retain(a)), TextOp::Retain(b)) => {
            *a = checked_sequence_add(*a, b)?;
        }
        (Some(TextOp::Delete(a)), TextOp::Delete(b)) => {
            *a = checked_sequence_add(*a, b)?;
        }
        (Some(TextOp::Insert(a)), TextOp::Insert(b)) => {
            let len = checked_sequence_add(a.len(), b.len())?;
            if len > isize::MAX as usize {
                return Err(ValueError::LengthOverflow);
            }
            a.push_str(&b);
        }
        (_, op) => out.push(op),
    }
    Ok(())
}

fn normalize_rich<I>(ops: I) -> Result<Vec<RichTextOp>, ValueError>
where
    I: IntoIterator<Item = RichTextOp>,
{
    let mut out = Vec::new();
    for op in ops {
        let op = match op {
            RichTextOp::Retain { len: 0, .. } | RichTextOp::Delete(0) => continue,
            RichTextOp::Insert { content, .. } if content.is_empty() => continue,
            other => other,
        };
        push_rich(&mut out, op)?;
    }
    while matches!(out.last(), Some(RichTextOp::Retain { attrs, .. }) if attrs.is_empty()) {
        out.pop();
    }
    Ok(out)
}

fn push_rich(out: &mut Vec<RichTextOp>, op: RichTextOp) -> Result<(), ValueError> {
    if matches!(op, RichTextOp::Insert { .. }) && matches!(out.last(), Some(RichTextOp::Delete(_)))
    {
        let delete = out.pop().unwrap();
        push_rich(out, op)?;
        push_rich(out, delete)?;
        return Ok(());
    }
    match (out.last_mut(), op) {
        (
            Some(RichTextOp::Retain { len: a, attrs: aa }),
            RichTextOp::Retain { len: b, attrs: ba },
        ) if *aa == ba => {
            *a = a.checked_add(b).ok_or(ValueError::LengthOverflow)?;
        }
        (Some(RichTextOp::Delete(a)), RichTextOp::Delete(b)) => {
            *a = a.checked_add(b).ok_or(ValueError::LengthOverflow)?;
        }
        (
            Some(RichTextOp::Insert {
                content: RichContent::Text(a),
                attrs: aa,
            }),
            RichTextOp::Insert {
                content: RichContent::Text(b),
                attrs: ba,
            },
        ) if *aa == ba => {
            *a = a.try_concat(&b)?;
        }
        (_, op) => out.push(op),
    }
    Ok(())
}
