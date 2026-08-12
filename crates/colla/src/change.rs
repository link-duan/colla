//! Canonical recursive Changes and typed Change constructors.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::attrs::{AttrPatch, Attrs};
use crate::error::{CodecError, ValueError};
use crate::input_limits::InputLimits;
use crate::richtext::RichContent;
use crate::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Deterministic ordering for conflicts between the left and right arguments
/// of `transform_pair`.
pub enum TieBreak {
    /// Give the left input precedence for otherwise unresolved conflicts.
    LeftFirst,
    /// Give the right input precedence for otherwise unresolved conflicts.
    RightFirst,
}

/// A checked integer addition Change.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IntChange {
    /// Adds the signed delta to an Int Snapshot.
    Add(i64),
}

/// A Change applied to one Map entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MapEntryChange {
    /// Inserts a key that is absent from the base Map.
    Insert(Value),
    /// Deletes a key that is present in the base Map.
    Delete,
    /// Recursively modifies a key that is present in the base Map.
    Modify(Change),
}

/// A canonical set of unique Map entry Changes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapChange(Arc<BTreeMap<String, MapEntryChange>>);

impl Hash for MapChange {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (key, value) in self.0.iter() {
            key.hash(state);
            value.hash(state);
        }
    }
}

impl MapChange {
    /// Creates a canonical Map Change and rejects duplicate keys.
    ///
    /// Entries are sorted by key, and `Modify(Noop)` entries are removed.
    pub fn from_entries<I, K>(entries: I) -> Result<Self, ValueError>
    where
        I: IntoIterator<Item = (K, MapEntryChange)>,
        K: Into<String>,
    {
        let mut map = BTreeMap::new();
        for (key, entry) in entries {
            let key = key.into();
            let entry = match entry {
                MapEntryChange::Modify(change) if change.is_noop() => continue,
                other => other,
            };
            if map.insert(key.clone(), entry).is_some() {
                return Err(ValueError::DuplicateKey(key));
            }
        }
        Ok(Self(Arc::new(map)))
    }
    pub(crate) fn from_btree(map: BTreeMap<String, MapEntryChange>) -> Self {
        Self(Arc::new(map))
    }
    /// Iterates entries in canonical key order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &MapEntryChange)> {
        self.0.iter()
    }
    /// Returns whether this typed Change contains no entries.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Returns the number of changed keys.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// An operation in a List Change stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ListOp {
    /// Retains this many base elements unchanged.
    Retain(usize),
    /// Inserts Values at the current position.
    Insert(Vec<Value>),
    /// Deletes this many base elements.
    Delete(usize),
    /// Recursively modifies the next base element and consumes one element.
    Modify(Change),
}

/// A canonical List operation stream.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ListChange(Arc<Vec<ListOp>>);

impl ListChange {
    /// Creates and normalizes a List operation stream.
    ///
    /// Zero operations and empty inserts are removed, adjacent operations are
    /// merged, inserts sort before deletes at one position, and trailing
    /// retains are omitted. Length and capacity overflow is rejected.
    pub fn from_ops<I>(ops: I) -> Result<Self, ValueError>
    where
        I: IntoIterator<Item = ListOp>,
    {
        let ops = normalize_list(ops)?;
        validate_list_lengths(&ops)?;
        Ok(Self(Arc::new(ops)))
    }
    pub(crate) fn from_canonical(ops: Vec<ListOp>) -> Self {
        Self(Arc::new(ops))
    }
    /// Returns the canonical operations.
    pub fn ops(&self) -> &[ListOp] {
        &self.0
    }
    /// Returns whether the canonical stream contains no operations.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// An operation in a Text Change stream.
///
/// Retain and Delete lengths count Unicode scalar values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextOp {
    /// Retains this many Unicode scalars unchanged.
    Retain(usize),
    /// Inserts UTF-8 text at the current position.
    Insert(String),
    /// Deletes this many Unicode scalars from the base Text.
    Delete(usize),
}

/// A canonical Text operation stream.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TextChange(Arc<Vec<TextOp>>);

impl TextChange {
    /// Creates and normalizes a Text operation stream.
    ///
    /// Zero operations and empty inserts are removed, compatible operations
    /// are merged, and trailing retains are omitted.
    ///
    /// # Examples
    ///
    /// ```
    /// use colla::{Change, TextChange, TextOp};
    ///
    /// let change: Change = TextChange::from_ops([
    ///     TextOp::Retain(2),
    ///     TextOp::Insert("!".into()),
    /// ])?
    /// .into();
    /// assert!(!change.is_noop());
    /// # Ok::<(), colla::ValueError>(())
    /// ```
    pub fn from_ops<I>(ops: I) -> Result<Self, ValueError>
    where
        I: IntoIterator<Item = TextOp>,
    {
        let ops = normalize_text(ops)?;
        validate_text_lengths(&ops)?;
        Ok(Self(Arc::new(ops)))
    }
    pub(crate) fn from_canonical(ops: Vec<TextOp>) -> Self {
        Self(Arc::new(ops))
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RichTextOp {
    /// Retains content and applies `attrs` to every retained scalar or embed.
    Retain {
        /// Logical length in Unicode scalars and atomic embeds.
        len: usize,
        /// Attribute changes; an empty patch is a plain retain.
        attrs: AttrPatch,
    },
    /// Inserts text or one atomic embed with the supplied attributes.
    Insert {
        /// Text or embed content.
        content: RichContent,
        /// Attributes assigned to the inserted content.
        attrs: Attrs,
    },
    /// Deletes this many Unicode scalars or atomic embeds.
    Delete(usize),
}

/// A canonical RichText operation stream.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
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
    pub(crate) fn from_canonical(ops: Vec<RichTextOp>) -> Self {
        Self(Arc::new(ops))
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// The closed recursive operation model. Values are observed through
/// `Change::kind`; construction goes through canonical constructors.
pub enum ChangeKind {
    /// Identity Change.
    Noop,
    /// Replaces the complete target Value, including its type.
    Replace(Value),
    /// Changes Map entries.
    Map(MapChange),
    /// Changes a List sequence.
    List(ListChange),
    /// Changes collaborative Text.
    Text(TextChange),
    /// Changes RichText content and attributes.
    RichText(RichTextChange),
    /// Changes an Int with checked addition.
    Int(IntChange),
}

/// An immutable, canonical recursive operation relative to a Snapshot.
///
/// Construction itself is Snapshot-independent. Compatibility with a concrete
/// Snapshot is checked by [`crate::apply`].
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Change(Arc<ChangeKind>);

impl std::fmt::Debug for Change {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for Change {
    fn default() -> Self {
        Self::noop()
    }
}

impl Change {
    /// Creates the identity Change.
    pub fn noop() -> Self {
        Self(Arc::new(ChangeKind::Noop))
    }
    /// Creates an atomic replacement Change.
    pub fn replace(value: Value) -> Self {
        Self(Arc::new(ChangeKind::Replace(value)))
    }
    /// Returns the concrete Change kind.
    pub fn kind(&self) -> &ChangeKind {
        &self.0
    }
    /// Returns whether this Change is the identity.
    pub fn is_noop(&self) -> bool {
        matches!(self.kind(), ChangeKind::Noop)
    }
    pub(crate) fn kind_name(&self) -> &'static str {
        match self.kind() {
            ChangeKind::Noop => "Noop",
            ChangeKind::Replace(_) => "Replace",
            ChangeKind::Map(_) => "Map",
            ChangeKind::List(_) => "List",
            ChangeKind::Text(_) => "Text",
            ChangeKind::RichText(_) => "RichText",
            ChangeKind::Int(_) => "Int",
        }
    }
    pub(crate) fn check_input_limits(&self, limits: &InputLimits) -> Result<(), CodecError> {
        let mut stack = vec![(self, 1usize)];
        let mut nodes = 0usize;
        while let Some((change, depth)) = stack.pop() {
            nodes += 1;
            check_limit("change nodes", nodes, limits.max_change_nodes)?;
            check_limit("change depth", depth, limits.max_depth)?;
            match change.kind() {
                ChangeKind::Noop | ChangeKind::Int(_) => {}
                ChangeKind::Replace(value) => value.check_input_limits(limits)?,
                ChangeKind::Map(map) => {
                    check_limit("container length", map.len(), limits.max_container_len)?;
                    for (key, entry) in map.iter() {
                        check_limit("string bytes", key.len(), limits.max_string_bytes)?;
                        match entry {
                            MapEntryChange::Insert(value) => value.check_input_limits(limits)?,
                            MapEntryChange::Delete => {}
                            MapEntryChange::Modify(child) => stack.push((child, depth + 1)),
                        }
                    }
                }
                ChangeKind::List(list) => {
                    check_limit("sequence ops", list.ops().len(), limits.max_sequence_ops)?;
                    let mut input_len = 0usize;
                    let mut output_len = 0usize;
                    for op in list.ops() {
                        match op {
                            ListOp::Insert(values) => {
                                check_limit(
                                    "container length",
                                    values.len(),
                                    limits.max_container_len,
                                )?;
                                for value in values {
                                    value.check_input_limits(limits)?;
                                }
                                output_len = add_len(output_len, values.len(), limits)?;
                            }
                            ListOp::Modify(child) => {
                                input_len = add_len(input_len, 1, limits)?;
                                output_len = add_len(output_len, 1, limits)?;
                                stack.push((child, depth + 1));
                            }
                            ListOp::Retain(len) => {
                                input_len = add_len(input_len, *len, limits)?;
                                output_len = add_len(output_len, *len, limits)?;
                            }
                            ListOp::Delete(len) => {
                                input_len = add_len(input_len, *len, limits)?;
                            }
                        }
                    }
                }
                ChangeKind::Text(text) => {
                    check_limit("sequence ops", text.ops().len(), limits.max_sequence_ops)?;
                    let mut input_len = 0usize;
                    let mut output_len = 0usize;
                    for op in text.ops() {
                        match op {
                            TextOp::Insert(value) => {
                                check_limit("string bytes", value.len(), limits.max_string_bytes)?;
                                output_len = add_len(output_len, value.chars().count(), limits)?;
                            }
                            TextOp::Retain(len) => {
                                input_len = add_len(input_len, *len, limits)?;
                                output_len = add_len(output_len, *len, limits)?;
                            }
                            TextOp::Delete(len) => {
                                input_len = add_len(input_len, *len, limits)?;
                            }
                        }
                    }
                }
                ChangeKind::RichText(rich) => {
                    check_limit("sequence ops", rich.ops().len(), limits.max_sequence_ops)?;
                    let mut input_len = 0usize;
                    let mut output_len = 0usize;
                    for op in rich.ops() {
                        match op {
                            RichTextOp::Retain { len, attrs } => {
                                input_len = add_len(input_len, *len, limits)?;
                                output_len = add_len(output_len, *len, limits)?;
                                check_attr_patch(attrs, limits)?;
                            }
                            RichTextOp::Insert { content, attrs } => {
                                check_attrs(attrs, limits)?;
                                output_len = add_len(output_len, content.len(), limits)?;
                                match content {
                                    RichContent::Text(text) => check_limit(
                                        "string bytes",
                                        text.as_str().len(),
                                        limits.max_string_bytes,
                                    )?,
                                    RichContent::Embed(value) => {
                                        value.check_input_limits(limits)?
                                    }
                                }
                            }
                            RichTextOp::Delete(len) => {
                                input_len = add_len(input_len, *len, limits)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl From<MapChange> for Change {
    fn from(change: MapChange) -> Self {
        if change.is_empty() {
            Self::noop()
        } else {
            Self(Arc::new(ChangeKind::Map(change)))
        }
    }
}

impl From<ListChange> for Change {
    fn from(change: ListChange) -> Self {
        if change.is_empty() {
            Self::noop()
        } else {
            Self(Arc::new(ChangeKind::List(change)))
        }
    }
}

impl From<TextChange> for Change {
    fn from(change: TextChange) -> Self {
        if change.is_empty() {
            Self::noop()
        } else {
            Self(Arc::new(ChangeKind::Text(change)))
        }
    }
}

impl From<RichTextChange> for Change {
    fn from(change: RichTextChange) -> Self {
        if change.is_empty() {
            Self::noop()
        } else {
            Self(Arc::new(ChangeKind::RichText(change)))
        }
    }
}

impl From<IntChange> for Change {
    fn from(change: IntChange) -> Self {
        match change {
            IntChange::Add(0) => Self::noop(),
            other => Self(Arc::new(ChangeKind::Int(other))),
        }
    }
}

fn check_attrs(attrs: &Attrs, limits: &InputLimits) -> Result<(), CodecError> {
    check_limit("container length", attrs.len(), limits.max_container_len)?;
    for (key, value) in attrs.iter() {
        check_limit("string bytes", key.len(), limits.max_string_bytes)?;
        if let crate::AttrValue::String(value) = value {
            check_limit("string bytes", value.len(), limits.max_string_bytes)?;
        }
    }
    Ok(())
}

fn check_attr_patch(patch: &AttrPatch, limits: &InputLimits) -> Result<(), CodecError> {
    check_limit("container length", patch.len(), limits.max_container_len)?;
    for (key, change) in patch.iter() {
        check_limit("string bytes", key.len(), limits.max_string_bytes)?;
        if let crate::AttrChange::Set(crate::AttrValue::String(value)) = change {
            check_limit("string bytes", value.len(), limits.max_string_bytes)?;
        }
    }
    Ok(())
}

fn check_limit(name: &'static str, actual: usize, limit: usize) -> Result<(), CodecError> {
    if actual > limit {
        Err(CodecError::LimitExceeded {
            name,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

fn add_len(current: usize, amount: usize, limits: &InputLimits) -> Result<usize, CodecError> {
    let value = current
        .checked_add(amount)
        .ok_or(CodecError::LimitExceeded {
            name: "sequence length",
            actual: usize::MAX,
            limit: limits.max_sequence_len,
        })?;
    check_limit("sequence length", value, limits.max_sequence_len)?;
    Ok(value)
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

fn validate_list_lengths(ops: &[ListOp]) -> Result<(), ValueError> {
    let mut lengths = ChangeLengths::default();
    for op in ops {
        match op {
            ListOp::Retain(len) => lengths.retain(*len)?,
            ListOp::Insert(values) => lengths.insert(values.len())?,
            ListOp::Delete(len) => lengths.delete(*len)?,
            ListOp::Modify(_) => lengths.retain(1)?,
        }
    }
    Ok(())
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

fn checked_vec_len<T>(left: usize, right: usize) -> Result<usize, ValueError> {
    let len = checked_sequence_add(left, right)?;
    let element_size = std::mem::size_of::<T>();
    if element_size != 0 && len > isize::MAX as usize / element_size {
        return Err(ValueError::LengthOverflow);
    }
    Ok(len)
}

fn normalize_list<I>(ops: I) -> Result<Vec<ListOp>, ValueError>
where
    I: IntoIterator<Item = ListOp>,
{
    let mut out = Vec::new();
    for op in ops {
        let op = match op {
            ListOp::Retain(0) | ListOp::Delete(0) => continue,
            ListOp::Insert(values) if values.is_empty() => continue,
            ListOp::Modify(change) if change.is_noop() => ListOp::Retain(1),
            other => other,
        };
        push_list(&mut out, op)?;
    }
    while matches!(out.last(), Some(ListOp::Retain(_))) {
        out.pop();
    }
    Ok(out)
}

fn push_list(out: &mut Vec<ListOp>, op: ListOp) -> Result<(), ValueError> {
    if matches!(op, ListOp::Insert(_)) && matches!(out.last(), Some(ListOp::Delete(_))) {
        let delete = out.pop().unwrap();
        push_list(out, op)?;
        push_list(out, delete)?;
        return Ok(());
    }
    match (out.last_mut(), op) {
        (Some(ListOp::Retain(a)), ListOp::Retain(b)) => {
            *a = checked_sequence_add(*a, b)?;
        }
        (Some(ListOp::Delete(a)), ListOp::Delete(b)) => {
            *a = checked_sequence_add(*a, b)?;
        }
        (Some(ListOp::Insert(a)), ListOp::Insert(mut b)) => {
            checked_vec_len::<Value>(a.len(), b.len())?;
            a.append(&mut b);
        }
        (_, op) => out.push(op),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_constructors_normalize_ops() {
        let text = TextChange::from_ops([
            TextOp::Retain(0),
            TextOp::Retain(1),
            TextOp::Retain(2),
            TextOp::Delete(1),
            TextOp::Insert(String::new()),
            TextOp::Insert("a".into()),
            TextOp::Insert("b".into()),
            TextOp::Retain(9),
        ])
        .unwrap();
        assert_eq!(
            text.ops(),
            &[
                TextOp::Retain(3),
                TextOp::Insert("ab".into()),
                TextOp::Delete(1),
            ]
        );

        let list = ListChange::from_ops([
            ListOp::Delete(1),
            ListOp::Insert(vec![Value::int(1)]),
            ListOp::Insert(vec![Value::int(2)]),
            ListOp::Modify(Change::noop()),
            ListOp::Retain(1),
        ])
        .unwrap();
        assert_eq!(
            list.ops(),
            &[
                ListOp::Insert(vec![Value::int(1), Value::int(2)]),
                ListOp::Delete(1),
            ]
        );

        let attrs = Attrs::new();
        let rich = RichTextChange::from_ops([
            RichTextOp::Delete(1),
            RichTextOp::Insert {
                content: RichContent::text("a"),
                attrs: attrs.clone(),
            },
            RichTextOp::Insert {
                content: RichContent::text("b"),
                attrs,
            },
            RichTextOp::Retain {
                len: 1,
                attrs: AttrPatch::new(),
            },
        ])
        .unwrap();
        assert_eq!(rich.ops().len(), 2);
        assert!(matches!(
            &rich.ops()[0],
            RichTextOp::Insert { content: RichContent::Text(text), .. }
                if text.as_str() == "ab"
        ));
        assert_eq!(rich.ops()[1], RichTextOp::Delete(1));
    }

    #[test]
    fn sequence_constructors_reject_all_length_accumulation_overflows() {
        assert_eq!(
            TextChange::from_ops([TextOp::Retain(usize::MAX), TextOp::Retain(1)]),
            Err(ValueError::LengthOverflow)
        );
        assert_eq!(
            TextChange::from_ops([TextOp::Retain(usize::MAX), TextOp::Insert("x".into()),]),
            Err(ValueError::LengthOverflow)
        );
        assert_eq!(
            ListChange::from_ops([
                ListOp::Delete(usize::MAX),
                ListOp::Modify(Change::replace(Value::null())),
            ]),
            Err(ValueError::LengthOverflow)
        );
        assert_eq!(
            RichTextChange::from_ops([
                RichTextOp::Retain {
                    len: usize::MAX,
                    attrs: AttrPatch::new(),
                },
                RichTextOp::Retain {
                    len: 1,
                    attrs: AttrPatch::from_entries([(
                        "bold",
                        crate::AttrChange::Set(crate::AttrValue::Bool(true)),
                    )])
                    .unwrap(),
                },
            ]),
            Err(ValueError::LengthOverflow)
        );
    }

    #[test]
    fn sequence_capacity_checks_use_rust_allocation_bound() {
        assert_eq!(
            checked_vec_len::<Value>(isize::MAX as usize, 1),
            Err(ValueError::LengthOverflow)
        );
        assert_eq!(
            checked_vec_len::<u8>(isize::MAX as usize, 1),
            Err(ValueError::LengthOverflow)
        );
    }
}
