//! Immutable Value types and Snapshot lookup.

use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::error::{CodecError, ValueError};
use crate::input_limits::InputLimits;
use crate::path::{Path, PathSeg};
use crate::richtext::RichText;

/// A finite, canonical `f64` value.
///
/// NaN and infinities are rejected, and negative zero is normalized to
/// positive zero so equality and canonical encoding remain deterministic.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FiniteF64(u64);

impl FiniteF64 {
    /// Creates a canonical finite float.
    pub fn new(value: f64) -> Result<Self, ValueError> {
        if !value.is_finite() {
            return Err(ValueError::NonFiniteFloat);
        }
        let canonical = if value == 0.0 { 0.0 } else { value };
        Ok(Self(canonical.to_bits()))
    }

    /// Returns the represented floating-point value.
    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

impl fmt::Debug for FiniteF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

/// Collaborative text addressed by Unicode scalar positions.
///
/// Unlike an atomic String Value, Text supports character-level OT.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Text(Arc<str>);

impl Text {
    /// Creates collaborative text from UTF-8 content.
    pub fn new(value: impl Into<String>) -> Self {
        Self(Arc::from(value.into()))
    }

    /// Returns the UTF-8 text content.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the Unicode scalar length.
    pub fn len(&self) -> usize {
        self.0.chars().count()
    }

    /// Returns whether the text contains no Unicode scalars.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// An immutable ordered collection of Values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct List(Arc<Vec<Value>>);

impl List {
    /// Creates a List from its elements.
    pub fn new(values: Vec<Value>) -> Self {
        Self(Arc::new(values))
    }

    /// Returns all elements as a borrowed slice.
    pub fn as_slice(&self) -> &[Value] {
        &self.0
    }

    /// Returns the number of elements.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the List contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the element at `index`, if present.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.0.get(index)
    }
}

/// An immutable map from unique string keys to Values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map(Arc<BTreeMap<String, Value>>);

impl Hash for Map {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (key, value) in self.0.iter() {
            key.hash(state);
            value.hash(state);
        }
    }
}

impl Map {
    /// Creates a Map and rejects duplicate keys.
    pub fn from_entries<I, K>(entries: I) -> Result<Self, ValueError>
    where
        I: IntoIterator<Item = (K, Value)>,
        K: Into<String>,
    {
        let mut map = BTreeMap::new();
        for (key, value) in entries {
            let key = key.into();
            if map.insert(key.clone(), value).is_some() {
                return Err(ValueError::DuplicateKey(key));
            }
        }
        Ok(Self(Arc::new(map)))
    }

    pub(crate) fn from_btree(map: BTreeMap<String, Value>) -> Self {
        Self(Arc::new(map))
    }

    /// Returns the Value associated with `key`, if present.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    /// Iterates entries in canonical key order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter()
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the Map contains no entries.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn to_btree(&self) -> BTreeMap<String, Value> {
        self.0.as_ref().clone()
    }
}

/// The closed set of Value type discriminants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    /// Null.
    Null,
    /// Boolean.
    Bool,
    /// Signed 64-bit integer.
    Int,
    /// Finite IEEE-754 `f64`.
    Float,
    /// Atomic UTF-8 string.
    String,
    /// Collaborative text.
    Text,
    /// Collaborative rich text.
    RichText,
    /// Ordered Value list.
    List,
    /// String-keyed Value map.
    Map,
}

/// The closed recursive content model stored by [`Value`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueKind {
    /// Null value.
    Null,
    /// Boolean value.
    Bool(bool),
    /// Signed 64-bit integer value.
    Int(i64),
    /// Canonical finite floating-point value.
    Float(FiniteF64),
    /// Atomic UTF-8 string value.
    String(Arc<str>),
    /// Collaborative text value.
    Text(Text),
    /// Collaborative RichText value.
    RichText(RichText),
    /// Ordered List value.
    List(List),
    /// String-keyed Map value.
    Map(Map),
}

/// An immutable, structurally shared Core Value.
///
/// A Value may be used as a complete Snapshot or nested inside another Value.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Value(Arc<ValueKind>);

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Value {
    /// Creates Null.
    pub fn null() -> Self {
        Self(Arc::new(ValueKind::Null))
    }
    /// Creates a Bool.
    pub fn bool(value: bool) -> Self {
        Self(Arc::new(ValueKind::Bool(value)))
    }
    /// Creates an Int.
    pub fn int(value: i64) -> Self {
        Self(Arc::new(ValueKind::Int(value)))
    }
    /// Creates a Float, rejecting NaN and infinities.
    pub fn float(value: f64) -> Result<Self, ValueError> {
        Ok(Self(Arc::new(ValueKind::Float(FiniteF64::new(value)?))))
    }
    /// Creates a Float from an already validated finite value.
    pub fn finite_float(value: FiniteF64) -> Self {
        Self(Arc::new(ValueKind::Float(value)))
    }
    /// Creates an atomic String.
    pub fn string(value: impl Into<String>) -> Self {
        Self(Arc::new(ValueKind::String(Arc::from(value.into()))))
    }
    /// Creates collaborative Text.
    pub fn text(value: impl Into<String>) -> Self {
        Self(Arc::new(ValueKind::Text(Text::new(value))))
    }
    /// Creates a RichText Value.
    pub fn rich_text(value: RichText) -> Self {
        Self(Arc::new(ValueKind::RichText(value)))
    }
    /// Creates a List from an iterator of Values.
    pub fn list<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
    {
        Self(Arc::new(ValueKind::List(List::new(
            values.into_iter().collect(),
        ))))
    }
    /// Creates a Map and rejects duplicate keys.
    pub fn map<I, K>(entries: I) -> Result<Self, ValueError>
    where
        I: IntoIterator<Item = (K, Value)>,
        K: Into<String>,
    {
        Ok(Self(Arc::new(ValueKind::Map(Map::from_entries(entries)?))))
    }
    pub(crate) fn from_kind(kind: ValueKind) -> Self {
        Self(Arc::new(kind))
    }

    /// Returns the concrete content kind.
    pub fn kind(&self) -> &ValueKind {
        &self.0
    }
    /// Returns the content type discriminant.
    pub fn value_type(&self) -> ValueType {
        match self.kind() {
            ValueKind::Null => ValueType::Null,
            ValueKind::Bool(_) => ValueType::Bool,
            ValueKind::Int(_) => ValueType::Int,
            ValueKind::Float(_) => ValueType::Float,
            ValueKind::String(_) => ValueType::String,
            ValueKind::Text(_) => ValueType::Text,
            ValueKind::RichText(_) => ValueType::RichText,
            ValueKind::List(_) => ValueType::List,
            ValueKind::Map(_) => ValueType::Map,
        }
    }

    /// Borrows the contained Map, if this Value is a Map.
    pub fn as_map(&self) -> Option<&Map> {
        if let ValueKind::Map(v) = self.kind() {
            Some(v)
        } else {
            None
        }
    }
    /// Borrows the contained List, if this Value is a List.
    pub fn as_list(&self) -> Option<&List> {
        if let ValueKind::List(v) = self.kind() {
            Some(v)
        } else {
            None
        }
    }
    /// Borrows the contained Text, if this Value is Text.
    pub fn as_text(&self) -> Option<&Text> {
        if let ValueKind::Text(v) = self.kind() {
            Some(v)
        } else {
            None
        }
    }
    /// Borrows the contained RichText, if this Value is RichText.
    pub fn as_rich_text(&self) -> Option<&RichText> {
        if let ValueKind::RichText(v) = self.kind() {
            Some(v)
        } else {
            None
        }
    }
    /// Returns the contained integer, if this Value is an Int.
    pub fn as_int(&self) -> Option<i64> {
        if let ValueKind::Int(v) = self.kind() {
            Some(*v)
        } else {
            None
        }
    }

    /// Resolves a Snapshot-relative Path and borrows the target Value.
    ///
    /// Paths navigate Map keys and List indexes only and are not part of a
    /// Change's canonical representation.
    pub fn get(&self, path: &Path) -> Option<&Value> {
        let mut current = self;
        for segment in path.segments() {
            current = match (current.kind(), segment) {
                (ValueKind::Map(map), PathSeg::Key(key)) => map.get(key)?,
                (ValueKind::List(list), PathSeg::Index(index)) => list.get(*index)?,
                _ => return None,
            };
        }
        Some(current)
    }

    pub(crate) fn check_input_limits(&self, limits: &InputLimits) -> Result<(), CodecError> {
        let mut stack = vec![(self, 1usize)];
        let mut nodes = 0usize;
        while let Some((value, depth)) = stack.pop() {
            nodes += 1;
            if nodes > limits.max_value_nodes {
                return Err(CodecError::LimitExceeded {
                    name: "value nodes",
                    actual: nodes,
                    limit: limits.max_value_nodes,
                });
            }
            if depth > limits.max_depth {
                return Err(CodecError::LimitExceeded {
                    name: "depth",
                    actual: depth,
                    limit: limits.max_depth,
                });
            }
            match value.kind() {
                ValueKind::String(s) => {
                    check_len("string bytes", s.len(), limits.max_string_bytes)?
                }
                ValueKind::Text(t) => {
                    check_len("string bytes", t.as_str().len(), limits.max_string_bytes)?
                }
                ValueKind::List(list) => {
                    check_len("container length", list.len(), limits.max_container_len)?;
                    for child in list.as_slice().iter().rev() {
                        stack.push((child, depth + 1));
                    }
                }
                ValueKind::Map(map) => {
                    check_len("container length", map.len(), limits.max_container_len)?;
                    for (key, child) in map.iter() {
                        check_len("string bytes", key.len(), limits.max_string_bytes)?;
                        stack.push((child, depth + 1));
                    }
                }
                ValueKind::RichText(rich) => {
                    check_len(
                        "container length",
                        rich.span_count(),
                        limits.max_container_len,
                    )?;
                    check_len("sequence length", rich.len(), limits.max_sequence_len)?;
                    for span in rich.iter_spans().rev() {
                        match span.content() {
                            crate::richtext::RichContent::Text(text) => {
                                check_len(
                                    "string bytes",
                                    text.as_str().len(),
                                    limits.max_string_bytes,
                                )?;
                            }
                            crate::richtext::RichContent::Embed(child) => {
                                stack.push((child, depth + 1));
                            }
                        }
                        check_len(
                            "container length",
                            span.attrs().len(),
                            limits.max_container_len,
                        )?;
                        for (key, value) in span.attrs().iter() {
                            check_len("string bytes", key.len(), limits.max_string_bytes)?;
                            if let crate::AttrValue::String(value) = value {
                                check_len("string bytes", value.len(), limits.max_string_bytes)?;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn check_len(name: &'static str, actual: usize, limit: usize) -> Result<(), CodecError> {
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
