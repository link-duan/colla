//! Lazily derived parent and back-reference lookups for one immutable subtree.

use super::{Body, Segment, Value};
use crate::engine::ElementId;
use std::collections::BTreeMap;

// Derived lazily per immutable subtree. Parent links avoid duplicating complete
// paths for descendants when an ancestor moves. Neither index is serialized.
#[derive(Debug, Default)]
pub(super) struct Index {
    pub(super) parents: BTreeMap<ElementId, (ElementId, Segment)>,
    pub(super) references: BTreeMap<ElementId, Vec<ElementId>>,
}
impl Index {
    pub(super) fn build(root: &Value) -> Self {
        let mut index = Self::default();
        root.visit(&mut |value| match value.body() {
            Body::Map(map) => {
                for (key, child) in map {
                    index
                        .parents
                        .insert(child.id(), (value.id(), Segment::Key(key.clone())));
                }
            }
            Body::List(list) => {
                for (position, child) in list.iter().enumerate() {
                    index
                        .parents
                        .insert(child.id(), (value.id(), Segment::Index(position)));
                }
            }
            _ => {}
        });
        root.visit_all(&mut |value| {
            if let Body::Ref(reference) = value.body() {
                index
                    .references
                    .entry(reference.target)
                    .or_default()
                    .push(value.id());
            }
        });
        index
    }
}
