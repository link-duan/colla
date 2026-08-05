use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::error::{ApplyError, ValueError};
use crate::limits::Limits;
use crate::path::{Path, PathSeg};
use crate::richtext::RichText;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FiniteF64(u64);

impl FiniteF64 {
    pub fn new(value: f64) -> Result<Self, ValueError> {
        if !value.is_finite() {
            return Err(ValueError::NonFiniteFloat);
        }
        let canonical = if value == 0.0 { 0.0 } else { value };
        Ok(Self(canonical.to_bits()))
    }

    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

impl fmt::Debug for FiniteF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Text(Arc<str>);

impl Text {
    pub fn new(value: impl Into<String>) -> Self {
        Self(Arc::from(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.chars().count()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct List(Arc<Vec<Value>>);

impl List {
    pub fn new(values: Vec<Value>) -> Self {
        Self(Arc::new(values))
    }

    pub fn as_slice(&self) -> &[Value] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&Value> {
        self.0.get(index)
    }
}

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

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn to_btree(&self) -> BTreeMap<String, Value> {
        self.0.as_ref().clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    Null,
    Bool,
    Int,
    Float,
    String,
    Text,
    RichText,
    List,
    Map,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueKind {
    Null,
    Bool(bool),
    Int(i64),
    Float(FiniteF64),
    String(Arc<str>),
    Text(Text),
    RichText(RichText),
    List(List),
    Map(Map),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Value(Arc<ValueKind>);

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Value {
    pub fn null() -> Self {
        Self(Arc::new(ValueKind::Null))
    }
    pub fn bool(value: bool) -> Self {
        Self(Arc::new(ValueKind::Bool(value)))
    }
    pub fn int(value: i64) -> Self {
        Self(Arc::new(ValueKind::Int(value)))
    }
    pub fn float(value: f64) -> Result<Self, ValueError> {
        Ok(Self(Arc::new(ValueKind::Float(FiniteF64::new(value)?))))
    }
    pub fn finite_float(value: FiniteF64) -> Self {
        Self(Arc::new(ValueKind::Float(value)))
    }
    pub fn string(value: impl Into<String>) -> Self {
        Self(Arc::new(ValueKind::String(Arc::from(value.into()))))
    }
    pub fn text(value: impl Into<String>) -> Self {
        Self(Arc::new(ValueKind::Text(Text::new(value))))
    }
    pub fn rich_text(value: RichText) -> Self {
        Self(Arc::new(ValueKind::RichText(value)))
    }
    pub fn list<I>(values: I) -> Self
    where
        I: IntoIterator<Item = Value>,
    {
        Self(Arc::new(ValueKind::List(List::new(
            values.into_iter().collect(),
        ))))
    }
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

    pub fn kind(&self) -> &ValueKind {
        &self.0
    }
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

    pub fn as_map(&self) -> Option<&Map> {
        if let ValueKind::Map(v) = self.kind() {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_list(&self) -> Option<&List> {
        if let ValueKind::List(v) = self.kind() {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_text(&self) -> Option<&Text> {
        if let ValueKind::Text(v) = self.kind() {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_rich_text(&self) -> Option<&RichText> {
        if let ValueKind::RichText(v) = self.kind() {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_int(&self) -> Option<i64> {
        if let ValueKind::Int(v) = self.kind() {
            Some(*v)
        } else {
            None
        }
    }

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

    pub fn check_limits(&self, limits: &Limits) -> Result<(), ApplyError> {
        let mut stack = vec![(self, 1usize)];
        let mut nodes = 0usize;
        while let Some((value, depth)) = stack.pop() {
            nodes += 1;
            if nodes > limits.max_value_nodes {
                return Err(ApplyError::LimitExceeded {
                    name: "value nodes",
                    actual: nodes,
                    limit: limits.max_value_nodes,
                });
            }
            if depth > limits.max_depth {
                return Err(ApplyError::LimitExceeded {
                    name: "value depth",
                    actual: depth,
                    limit: limits.max_depth,
                });
            }
            match value.kind() {
                ValueKind::String(s) => {
                    check_len("string bytes", s.len(), limits.max_string_bytes)?
                }
                ValueKind::Text(t) => {
                    check_len("text bytes", t.as_str().len(), limits.max_string_bytes)?
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
                        rich.spans().len(),
                        limits.max_container_len,
                    )?;
                    for span in rich.spans().iter().rev() {
                        match &span.content {
                            crate::richtext::RichInsert::Text(text) => {
                                check_len("string bytes", text.len(), limits.max_string_bytes)?;
                            }
                            crate::richtext::RichInsert::Embed(child) => {
                                stack.push((child, depth + 1));
                            }
                        }
                        check_len(
                            "container length",
                            span.attrs.len(),
                            limits.max_container_len,
                        )?;
                        for (key, value) in span.attrs.iter() {
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

fn check_len(name: &'static str, actual: usize, limit: usize) -> Result<(), ApplyError> {
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
