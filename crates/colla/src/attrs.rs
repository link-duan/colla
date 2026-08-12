//! RichText attribute values, sets, and formatting patches.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::error::ValueError;
use crate::value::FiniteF64;

/// An atomic RichText attribute value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttrValue {
    /// Boolean attribute value.
    Bool(bool),
    /// Signed 64-bit integer attribute value.
    Int(i64),
    /// Canonical finite floating-point attribute value.
    Float(FiniteF64),
    /// UTF-8 string attribute value.
    String(Arc<str>),
}

impl AttrValue {
    /// Creates a finite floating-point attribute value.
    pub fn float(value: f64) -> Result<Self, ValueError> {
        Ok(Self::Float(FiniteF64::new(value)?))
    }
    /// Creates a string attribute value.
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(Arc::from(value.into()))
    }
}

impl From<bool> for AttrValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl From<i64> for AttrValue {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}
impl From<String> for AttrValue {
    fn from(value: String) -> Self {
        Self::String(Arc::from(value))
    }
}
impl From<&str> for AttrValue {
    fn from(value: &str) -> Self {
        Self::String(Arc::from(value))
    }
}

/// An immutable, canonically ordered set of RichText attributes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attrs(Arc<BTreeMap<String, AttrValue>>);

impl Hash for Attrs {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (key, value) in self.0.iter() {
            key.hash(state);
            value.hash(state);
        }
    }
}

impl Attrs {
    /// Creates an empty attribute set.
    pub fn new() -> Self {
        Self::default()
    }
    /// Creates an attribute set and rejects duplicate keys.
    pub fn from_entries<I, K>(entries: I) -> Result<Self, ValueError>
    where
        I: IntoIterator<Item = (K, AttrValue)>,
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
    pub(crate) fn from_btree(map: BTreeMap<String, AttrValue>) -> Self {
        Self(Arc::new(map))
    }
    /// Returns the value assigned to `key`, if present.
    pub fn get(&self, key: &str) -> Option<&AttrValue> {
        self.0.get(key)
    }
    /// Iterates attributes in canonical key order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &AttrValue)> {
        self.0.iter()
    }
    /// Returns whether no attributes are set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Returns the number of attributes.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub(crate) fn to_btree(&self) -> BTreeMap<String, AttrValue> {
        self.0.as_ref().clone()
    }
    /// Applies a Set/Remove patch and returns a new attribute set.
    pub fn apply_patch(&self, patch: &AttrPatch) -> Self {
        let mut out = self.to_btree();
        for (key, change) in patch.iter() {
            match change {
                AttrChange::Set(value) => {
                    out.insert(key.clone(), value.clone());
                }
                AttrChange::Remove => {
                    out.remove(key);
                }
            }
        }
        Self::from_btree(out)
    }
}

/// A change to one RichText attribute key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttrChange {
    /// Assigns or overwrites the attribute value.
    Set(AttrValue),
    /// Removes the attribute if present.
    Remove,
}

/// An immutable, canonically ordered RichText attribute patch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttrPatch(Arc<BTreeMap<String, AttrChange>>);

impl Hash for AttrPatch {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (key, value) in self.0.iter() {
            key.hash(state);
            value.hash(state);
        }
    }
}

impl AttrPatch {
    /// Creates an empty patch.
    pub fn new() -> Self {
        Self::default()
    }
    /// Creates a patch and rejects duplicate keys.
    pub fn from_entries<I, K>(entries: I) -> Result<Self, ValueError>
    where
        I: IntoIterator<Item = (K, AttrChange)>,
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
    pub(crate) fn from_btree(map: BTreeMap<String, AttrChange>) -> Self {
        Self(Arc::new(map))
    }
    /// Iterates attribute changes in canonical key order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &AttrChange)> {
        self.0.iter()
    }
    /// Returns whether the patch changes no keys.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Returns the number of changed keys.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub(crate) fn to_btree(&self) -> BTreeMap<String, AttrChange> {
        self.0.as_ref().clone()
    }
}
