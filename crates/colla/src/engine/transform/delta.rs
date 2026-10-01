//! Identity-preserving delta construction for transformed results.

use super::super::{
    change::{apply_operation, destination_of, structural},
    Body, Change, Destination, ElementId, Operation, Result, Segment, Value,
};
use super::LeafChanges;
use crate::sequence::change::{TextChange, TextOp};
use std::collections::{BTreeMap, BTreeSet};

/// Builds an executable identity-aware delta. Reordering is expressed with
/// native moves, including temporary parking when Map keys form a permutation.
pub(super) fn difference(
    before: &Value,
    after: &Value,
    leaves: LeafChanges,
    original: &Change,
) -> Result<Change> {
    if before.id() != after.id() {
        return Err(structural("document root identity changed"));
    }
    let wanted = after.ids();
    let previous = before.ids();
    let mut movable: BTreeSet<_> = wanted.difference(&previous).copied().collect();
    for operation in original.operations() {
        if let Operation::Move { target, .. } = operation {
            if wanted.contains(target) {
                movable.insert(*target);
            }
        }
    }
    let mut state = Delta {
        value: before.clone(),
        operations: Vec::new(),
        leaves,
        movable,
    };
    state.prune(before, &wanted)?;
    state.align(after)?;
    state.clean(after)?;
    if state.value != *after {
        return Err(structural("cannot construct identity-preserving delta"));
    }
    Ok(Change::new(state.operations)?.normalized(before)?.0)
}
struct Delta {
    value: Value,
    operations: Vec<Operation>,
    leaves: LeafChanges,
    movable: BTreeSet<ElementId>,
}
impl Delta {
    fn prune(&mut self, value: &Value, wanted: &BTreeSet<ElementId>) -> Result<()> {
        if !wanted.contains(&value.id()) && value.ids().is_disjoint(wanted) {
            return self.push(Operation::Delete { target: value.id() });
        }
        match value.body() {
            Body::Map(map) => {
                for child in map.values() {
                    self.prune(child, wanted)?;
                }
            }
            Body::List(list) => {
                for child in list {
                    self.prune(child, wanted)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn push(&mut self, operation: Operation) -> Result<()> {
        let next = apply_operation(&self.value, &operation)?;
        if next != self.value {
            self.operations.push(operation);
        }
        self.value = next;
        Ok(())
    }
    fn park(&mut self, id: ElementId) -> Result<()> {
        let slot = match self.value.body() {
            Body::Map(map) => {
                let mut n = 0u64;
                loop {
                    let key = format!("\0colla:{n}");
                    if !map.contains_key(&key) {
                        break Segment::Key(key);
                    }
                    n += 1;
                }
            }
            Body::List(list) => Segment::Index(
                list.len()
                    - usize::from(destination_of(&self.value, id)?.parent == self.value.id()),
            ),
            _ => return Err(structural("cannot park an element under a scalar root")),
        };
        self.push(Operation::Move {
            target: id,
            destination: Destination {
                parent: self.value.id(),
                slot,
            },
        })
    }
    fn place(&mut self, desired: &Value, destination: Destination) -> Result<()> {
        if let Segment::Key(key) = &destination.slot {
            let parent = self.value.get(destination.parent)?;
            if let Body::Map(map) = parent.body() {
                if let Some(occupied) = map.get(key) {
                    if occupied.id() != desired.id() {
                        self.park(occupied.id())?;
                    }
                }
            }
        }
        if self.value.find(desired.id()).is_some() {
            if destination_of(&self.value, desired.id())? != destination {
                self.push(Operation::Move {
                    target: desired.id(),
                    destination,
                })?;
            }
        } else {
            let body = match desired.body() {
                Body::Map(_) => Body::Map(BTreeMap::new()),
                Body::List(_) => Body::List(Vec::new()),
                body => body.clone(),
            };
            self.push(Operation::Insert {
                destination,
                value: Value::trusted(desired.id(), body),
            })?;
        }
        self.align(desired)
    }
    fn align(&mut self, desired: &Value) -> Result<()> {
        let current = self.value.get(desired.id())?;
        if current != *desired {
            if let Some(operation) = self.leaves.remove(&desired.id()) {
                if let Ok(after) = apply_operation(&self.value, &operation) {
                    if after.find(desired.id()) == Some(desired) {
                        return self.push(operation);
                    }
                }
            }
        }
        match (current.body(), desired.body()) {
            (Body::Map(_), Body::Map(map)) => {
                for (key, child) in map {
                    self.place(
                        child,
                        Destination {
                            parent: desired.id(),
                            slot: Segment::Key(key.clone()),
                        },
                    )?;
                }
            }
            (Body::List(_), Body::List(list)) => {
                for (index, child) in list.iter().enumerate() {
                    if !self.movable.contains(&child.id())
                        && destination_of(&self.value, child.id())
                            .is_ok_and(|d| d.parent == desired.id())
                    {
                        self.align(child)?;
                        continue;
                    }
                    let parent = self.value.get(desired.id())?;
                    let Body::List(current) = parent.body() else {
                        unreachable!()
                    };
                    let current: Vec<_> = current.iter().filter(|v| v.id() != child.id()).collect();
                    let position = list[index + 1..]
                        .iter()
                        .filter(|anchor| !self.movable.contains(&anchor.id()))
                        .find_map(|anchor| current.iter().position(|v| v.id() == anchor.id()))
                        .unwrap_or(current.len());
                    self.place(
                        child,
                        Destination {
                            parent: desired.id(),
                            slot: Segment::Index(position),
                        },
                    )?;
                }
            }
            (Body::Text(a), Body::Text(b)) if a != b => {
                let a: Vec<_> = a.chars().collect();
                let b: Vec<_> = b.chars().collect();
                let prefix = a.iter().zip(&b).take_while(|(a, b)| a == b).count();
                let suffix = a[prefix..]
                    .iter()
                    .rev()
                    .zip(b[prefix..].iter().rev())
                    .take_while(|(a, b)| a == b)
                    .count();
                self.push(Operation::Text {
                    target: desired.id(),
                    change: TextChange::from_ops([
                        TextOp::Retain(prefix),
                        TextOp::Delete(a.len() - prefix - suffix),
                        TextOp::Insert(b[prefix..b.len() - suffix].iter().collect()),
                    ])?,
                })?;
            }
            (Body::Int(a), Body::Int(b)) if a != b => {
                let mut delta = i128::from(*b) - i128::from(*a);
                while delta != 0 {
                    let step = delta.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
                    self.push(Operation::Add {
                        target: desired.id(),
                        delta: step,
                    })?;
                    delta -= i128::from(step);
                }
            }
            _ if current != *desired => self.push(Operation::Set {
                target: desired.id(),
                value: desired.clone(),
            })?,
            _ => {}
        }
        Ok(())
    }
    fn clean(&mut self, desired: &Value) -> Result<()> {
        let current = self.value.get(desired.id())?;
        match (current.body(), desired.body()) {
            (Body::Map(map), Body::Map(wanted)) => {
                for (key, child) in map {
                    if !wanted.contains_key(key) {
                        self.push(Operation::Delete { target: child.id() })?;
                    }
                }
                for child in wanted.values() {
                    self.clean(child)?;
                }
            }
            (Body::List(list), Body::List(wanted)) => {
                for child in list.iter().skip(wanted.len()) {
                    self.push(Operation::Delete { target: child.id() })?;
                }
                for child in wanted {
                    self.clean(child)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
