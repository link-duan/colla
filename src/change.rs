use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::attrs::{AttrPatch, Attrs};
use crate::error::{ApplyError, ValueError};
use crate::limits::Limits;
use crate::richtext::RichInsert;
use crate::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Deterministic ordering for conflicts between the left and right arguments
/// of `transform_pair`.
pub enum TieBreak {
    LeftFirst,
    RightFirst,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IntChange {
    Add(i64),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MapEntryChange {
    Insert(Value),
    Delete,
    Modify(Change),
}

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
    pub fn iter(&self) -> impl Iterator<Item = (&String, &MapEntryChange)> {
        self.0.iter()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ListOp {
    Retain(usize),
    Insert(Vec<Value>),
    Delete(usize),
    Modify(Change),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ListChange(Arc<Vec<ListOp>>);

impl ListChange {
    pub fn new(ops: Vec<ListOp>) -> Self {
        Self(Arc::new(normalize_list(ops)))
    }
    pub(crate) fn from_canonical(ops: Vec<ListOp>) -> Self {
        Self(Arc::new(ops))
    }
    pub fn ops(&self) -> &[ListOp] {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextOp {
    Retain(usize),
    Insert(String),
    Delete(usize),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TextChange(Arc<Vec<TextOp>>);

impl TextChange {
    pub fn new(ops: Vec<TextOp>) -> Self {
        Self(Arc::new(normalize_text(ops)))
    }
    pub(crate) fn from_canonical(ops: Vec<TextOp>) -> Self {
        Self(Arc::new(ops))
    }
    pub fn ops(&self) -> &[TextOp] {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A rich-text retain keeps `len` sequence units and applies `attrs` to each
/// retained character or embed. An empty patch is a plain retain.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RichTextOp {
    Retain { len: usize, attrs: AttrPatch },
    Insert { content: RichInsert, attrs: Attrs },
    Delete(usize),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct RichTextChange(Arc<Vec<RichTextOp>>);

impl RichTextChange {
    pub fn new(ops: Vec<RichTextOp>) -> Self {
        Self(Arc::new(normalize_rich(ops)))
    }
    pub(crate) fn from_canonical(ops: Vec<RichTextOp>) -> Self {
        Self(Arc::new(ops))
    }
    pub fn ops(&self) -> &[RichTextOp] {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// The closed recursive operation model. Values are observed through
/// `Change::kind`; construction goes through canonical constructors.
pub enum ChangeKind {
    Noop,
    Replace(Value),
    Map(MapChange),
    List(ListChange),
    Text(TextChange),
    RichText(RichTextChange),
    Int(IntChange),
}

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
    pub fn noop() -> Self {
        Self(Arc::new(ChangeKind::Noop))
    }
    pub fn replace(value: Value) -> Self {
        Self(Arc::new(ChangeKind::Replace(value)))
    }
    pub fn map(change: MapChange) -> Self {
        if change.is_empty() {
            Self::noop()
        } else {
            Self(Arc::new(ChangeKind::Map(change)))
        }
    }
    pub fn list(change: ListChange) -> Self {
        if change.is_empty() {
            Self::noop()
        } else {
            Self(Arc::new(ChangeKind::List(change)))
        }
    }
    pub fn text(change: TextChange) -> Self {
        if change.is_empty() {
            Self::noop()
        } else {
            Self(Arc::new(ChangeKind::Text(change)))
        }
    }
    pub fn rich_text(change: RichTextChange) -> Self {
        if change.is_empty() {
            Self::noop()
        } else {
            Self(Arc::new(ChangeKind::RichText(change)))
        }
    }
    pub fn int_add(delta: i64) -> Self {
        if delta == 0 {
            Self::noop()
        } else {
            Self(Arc::new(ChangeKind::Int(IntChange::Add(delta))))
        }
    }
    pub fn kind(&self) -> &ChangeKind {
        &self.0
    }
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
    pub(crate) fn check_limits(&self, limits: &Limits) -> Result<(), ApplyError> {
        let mut stack = vec![(self, 1usize)];
        let mut nodes = 0usize;
        while let Some((change, depth)) = stack.pop() {
            nodes += 1;
            check_limit("change nodes", nodes, limits.max_change_nodes)?;
            check_limit("change depth", depth, limits.max_depth)?;
            match change.kind() {
                ChangeKind::Noop | ChangeKind::Int(_) => {}
                ChangeKind::Replace(value) => value.check_limits(limits)?,
                ChangeKind::Map(map) => {
                    check_limit("container length", map.len(), limits.max_container_len)?;
                    for (key, entry) in map.iter() {
                        check_limit("string bytes", key.len(), limits.max_string_bytes)?;
                        match entry {
                            MapEntryChange::Insert(value) => value.check_limits(limits)?,
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
                                    value.check_limits(limits)?;
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
                                    RichInsert::Text(text) => check_limit(
                                        "string bytes",
                                        text.len(),
                                        limits.max_string_bytes,
                                    )?,
                                    RichInsert::Embed(value) => value.check_limits(limits)?,
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

fn check_attrs(attrs: &Attrs, limits: &Limits) -> Result<(), ApplyError> {
    check_limit("container length", attrs.len(), limits.max_container_len)?;
    for (key, value) in attrs.iter() {
        check_limit("string bytes", key.len(), limits.max_string_bytes)?;
        if let crate::AttrValue::String(value) = value {
            check_limit("string bytes", value.len(), limits.max_string_bytes)?;
        }
    }
    Ok(())
}

fn check_attr_patch(patch: &AttrPatch, limits: &Limits) -> Result<(), ApplyError> {
    check_limit("container length", patch.len(), limits.max_container_len)?;
    for (key, change) in patch.iter() {
        check_limit("string bytes", key.len(), limits.max_string_bytes)?;
        if let crate::AttrChange::Set(crate::AttrValue::String(value)) = change {
            check_limit("string bytes", value.len(), limits.max_string_bytes)?;
        }
    }
    Ok(())
}

fn check_limit(name: &'static str, actual: usize, limit: usize) -> Result<(), ApplyError> {
    if actual > limit {
        Err(ApplyError::LimitExceeded {
            name,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

fn add_len(current: usize, amount: usize, limits: &Limits) -> Result<usize, ApplyError> {
    let value = current
        .checked_add(amount)
        .ok_or(ApplyError::LimitExceeded {
            name: "sequence length",
            actual: usize::MAX,
            limit: limits.max_sequence_len,
        })?;
    check_limit("sequence length", value, limits.max_sequence_len)?;
    Ok(value)
}

fn normalize_list(ops: Vec<ListOp>) -> Vec<ListOp> {
    let mut out = Vec::new();
    for op in ops {
        let op = match op {
            ListOp::Retain(0) | ListOp::Delete(0) => continue,
            ListOp::Insert(values) if values.is_empty() => continue,
            ListOp::Modify(change) if change.is_noop() => ListOp::Retain(1),
            other => other,
        };
        push_list(&mut out, op);
    }
    while matches!(out.last(), Some(ListOp::Retain(_))) {
        out.pop();
    }
    out
}

fn push_list(out: &mut Vec<ListOp>, op: ListOp) {
    if matches!(op, ListOp::Insert(_)) && matches!(out.last(), Some(ListOp::Delete(_))) {
        let delete = out.pop().unwrap();
        push_list(out, op);
        push_list(out, delete);
        return;
    }
    match (out.last_mut(), op) {
        (Some(ListOp::Retain(a)), ListOp::Retain(b)) => *a += b,
        (Some(ListOp::Delete(a)), ListOp::Delete(b)) => *a += b,
        (Some(ListOp::Insert(a)), ListOp::Insert(mut b)) => a.append(&mut b),
        (_, op) => out.push(op),
    }
}

fn normalize_text(ops: Vec<TextOp>) -> Vec<TextOp> {
    let mut out = Vec::new();
    for op in ops {
        let op = match op {
            TextOp::Retain(0) | TextOp::Delete(0) => continue,
            TextOp::Insert(value) if value.is_empty() => continue,
            other => other,
        };
        push_text(&mut out, op);
    }
    while matches!(out.last(), Some(TextOp::Retain(_))) {
        out.pop();
    }
    out
}

fn push_text(out: &mut Vec<TextOp>, op: TextOp) {
    if matches!(op, TextOp::Insert(_)) && matches!(out.last(), Some(TextOp::Delete(_))) {
        let delete = out.pop().unwrap();
        push_text(out, op);
        push_text(out, delete);
        return;
    }
    match (out.last_mut(), op) {
        (Some(TextOp::Retain(a)), TextOp::Retain(b)) => *a += b,
        (Some(TextOp::Delete(a)), TextOp::Delete(b)) => *a += b,
        (Some(TextOp::Insert(a)), TextOp::Insert(b)) => a.push_str(&b),
        (_, op) => out.push(op),
    }
}

fn normalize_rich(ops: Vec<RichTextOp>) -> Vec<RichTextOp> {
    let mut out = Vec::new();
    for op in ops {
        let op = match op {
            RichTextOp::Retain { len: 0, .. } | RichTextOp::Delete(0) => continue,
            RichTextOp::Insert { content, .. } if content.is_empty() => continue,
            other => other,
        };
        push_rich(&mut out, op);
    }
    while matches!(out.last(), Some(RichTextOp::Retain { attrs, .. }) if attrs.is_empty()) {
        out.pop();
    }
    out
}

fn push_rich(out: &mut Vec<RichTextOp>, op: RichTextOp) {
    if matches!(op, RichTextOp::Insert { .. }) && matches!(out.last(), Some(RichTextOp::Delete(_)))
    {
        let delete = out.pop().unwrap();
        push_rich(out, op);
        push_rich(out, delete);
        return;
    }
    match (out.last_mut(), op) {
        (
            Some(RichTextOp::Retain { len: a, attrs: aa }),
            RichTextOp::Retain { len: b, attrs: ba },
        ) if *aa == ba => *a += b,
        (Some(RichTextOp::Delete(a)), RichTextOp::Delete(b)) => *a += b,
        (
            Some(RichTextOp::Insert {
                content: RichInsert::Text(a),
                attrs: aa,
            }),
            RichTextOp::Insert {
                content: RichInsert::Text(b),
                attrs: ba,
            },
        ) if *aa == ba => {
            let mut merged = String::from(a.as_ref());
            merged.push_str(&b);
            *a = Arc::from(merged);
        }
        (_, op) => out.push(op),
    }
}
