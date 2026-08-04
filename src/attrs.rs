use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::error::ValueError;
use crate::value::FiniteF64;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttrValue {
    Bool(bool),
    Int(i64),
    Float(FiniteF64),
    String(Arc<str>),
}

impl AttrValue {
    pub fn float(value: f64) -> Result<Self, ValueError> {
        Ok(Self::Float(FiniteF64::new(value)?))
    }
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
    pub fn new() -> Self {
        Self::default()
    }
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
    pub fn get(&self, key: &str) -> Option<&AttrValue> {
        self.0.get(key)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&String, &AttrValue)> {
        self.0.iter()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub(crate) fn to_btree(&self) -> BTreeMap<String, AttrValue> {
        self.0.as_ref().clone()
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttrChange {
    Set(AttrValue),
    Remove,
}

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
    pub fn new() -> Self {
        Self::default()
    }
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
    pub fn iter(&self) -> impl Iterator<Item = (&String, &AttrChange)> {
        self.0.iter()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub(crate) fn to_btree(&self) -> BTreeMap<String, AttrChange> {
        self.0.as_ref().clone()
    }
}
