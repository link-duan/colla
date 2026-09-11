use super::{
    change::{apply_operation, destination_of, structural},
    Body, Change, Destination, ElementId, Error, ErrorCode, Operation, Result, Segment, Value,
};
use crate::sequence::change::{TextChange, TextOp, TieBreak};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Deterministic precedence for otherwise competing concurrent intents.
pub enum Priority {
    /// Give the left concurrent input precedence.
    Left,
    /// Give the right concurrent input precedence.
    Right,
}

/// Returns (left after right, right after left), relative to `base`.
pub fn transform(
    base: &Value,
    left: &Change,
    right: &Change,
    priority: Priority,
) -> Result<(Change, Change)> {
    let (left, left_value) = left.normalized(base)?;
    let (right, right_value) = right.normalized(base)?;
    if let (Ok((left_after, a)), Ok((right_after, b))) =
        (left.normalized(&right_value), right.normalized(&left_value))
    {
        if a == b {
            return Ok((left_after, right_after));
        }
    }
    check_move_destinations(base, &left, &left_value, &right_value)?;
    check_move_destinations(base, &right, &right_value, &left_value)?;
    let merged = match priority {
        Priority::Left => merge(base, &left, &left_value, &right, &right_value)?,
        Priority::Right => merge(base, &right, &right_value, &left, &left_value)?,
    };
    let (left_leaves, right_leaves) = transformed_leaves(base, &left, &right, priority)?;
    Ok((
        difference(&right_value, &merged, left_leaves, &left)?,
        difference(&left_value, &merged, right_leaves, &right)?,
    ))
}

fn check_move_destinations(
    base: &Value,
    own: &Change,
    own_value: &Value,
    other: &Value,
) -> Result<()> {
    for (source, _) in intents(own)?.moved {
        if own_value.find(source).is_none()
            || (base.find(source).is_some() && other.find(source).is_none())
        {
            continue;
        }
        let destination = destination_of(own_value, source)?;
        if base.find(destination.parent).is_some() {
            let parent = other
                .find(destination.parent)
                .ok_or_else(|| structural("move destination parent was deleted"))?;
            if !matches!(
                (parent.body(), destination.slot),
                (Body::Map(_), Segment::Key(_)) | (Body::List(_), Segment::Index(_))
            ) {
                return Err(structural("move destination parent changed kind"));
            }
        }
    }
    Ok(())
}

fn preserve_incoming(current: &Value, replacement: &Value) -> Result<Value> {
    let body = match (current.body(), replacement.body()) {
        (Body::Map(incoming), Body::Map(new)) => {
            let mut map = incoming.clone();
            for (key, value) in new {
                if map.get(key).is_some_and(|old| old.id() != value.id()) {
                    return Err(structural("Set and incoming element compete for a Map key"));
                }
                map.insert(key.clone(), value.clone());
            }
            Body::Map(map)
        }
        (Body::List(incoming), Body::List(new)) => {
            let new_ids: BTreeSet<_> = new.iter().map(Value::id).collect();
            Body::List(
                incoming
                    .iter()
                    .filter(|v| !new_ids.contains(&v.id()))
                    .cloned()
                    .chain(new.iter().cloned())
                    .collect(),
            )
        }
        (Body::Map(incoming), _) if !incoming.is_empty() => {
            return Err(structural(
                "Set would remove a concurrently incoming element",
            ))
        }
        (Body::List(incoming), _) if !incoming.is_empty() => {
            return Err(structural(
                "Set would remove a concurrently incoming element",
            ))
        }
        (_, body) => body.clone(),
    };
    Value::with_id(replacement.id(), body)
}

type LeafChanges = BTreeMap<ElementId, Operation>;
fn transformed_leaves(
    base: &Value,
    left: &Change,
    right: &Change,
    priority: Priority,
) -> Result<(LeafChanges, LeafChanges)> {
    let left = intents(left)?;
    let right = intents(right)?;
    let mut ids = BTreeSet::new();
    ids.extend(
        left.text
            .keys()
            .chain(left.rich.keys())
            .chain(right.text.keys())
            .chain(right.rich.keys())
            .copied(),
    );
    let mut left_out = BTreeMap::new();
    let mut right_out = BTreeMap::new();
    for id in ids {
        if base.find(id).is_none() || left.set.contains(&id) || right.set.contains(&id) {
            continue;
        }
        let l = left
            .text
            .get(&id)
            .or_else(|| left.rich.get(&id))
            .cloned()
            .unwrap_or_default();
        let r = right
            .text
            .get(&id)
            .or_else(|| right.rich.get(&id))
            .cloned()
            .unwrap_or_default();
        let (l, r) = crate::sequence::op::transform(
            &l,
            &r,
            if priority == Priority::Left {
                TieBreak::LeftFirst
            } else {
                TieBreak::RightFirst
            },
        )
        .map_err(algebra_error)?;
        for (change, out) in [(l, &mut left_out), (r, &mut right_out)] {
            if let Some(text) = change.as_text() {
                out.insert(
                    id,
                    Operation::Text {
                        target: id,
                        change: text.clone(),
                    },
                );
            } else if change.as_rich_text().is_some() {
                out.insert(
                    id,
                    Operation::RichText {
                        target: id,
                        operations: super::rich::from_old(&change)?,
                    },
                );
            }
        }
    }
    Ok((left_out, right_out))
}

#[derive(Default)]
struct Intents {
    moved: BTreeMap<ElementId, usize>,
    set: BTreeSet<ElementId>,
    text: BTreeMap<ElementId, crate::sequence::change::Change>,
    added: BTreeMap<ElementId, i128>,
    rich: BTreeMap<ElementId, crate::sequence::change::Change>,
}
fn intents(change: &Change) -> Result<Intents> {
    let mut result = Intents::default();
    for (index, op) in change.operations().iter().enumerate() {
        match op {
            Operation::Move { target, .. } => {
                result.moved.insert(*target, index);
            }
            Operation::Set { target, .. } => {
                result.set.insert(*target);
                result.text.remove(target);
                result.added.remove(target);
                result.rich.remove(target);
            }
            Operation::Text { target, change } => {
                let previous = result.text.entry(*target).or_default();
                *previous = crate::sequence::op::compose(previous, &change.clone().into())
                    .map_err(algebra_error)?;
            }
            Operation::Add { target, delta } => {
                *result.added.entry(*target).or_default() += i128::from(*delta);
            }
            Operation::RichText { target, operations } => {
                let previous = result.rich.entry(*target).or_default();
                *previous =
                    crate::sequence::op::compose(previous, &super::rich::to_old(operations)?)
                        .map_err(algebra_error)?;
            }
            _ => {}
        }
    }
    Ok(result)
}

fn merge(
    base: &Value,
    high: &Change,
    high_value: &Value,
    low: &Change,
    low_value: &Value,
) -> Result<Value> {
    let hi = intents(high)?;
    let lo = intents(low)?;
    let mut merged = high_value.clone();
    let mut low_before = base.clone();
    for (index, operation) in low.operations().iter().enumerate() {
        match operation {
            Operation::Insert { destination, value } => {
                if merged.find(destination.parent).is_none() {
                    low_before = apply_operation(&low_before, operation)?;
                    continue;
                }
                let destination = rebase_destination(&low_before, &merged, destination, None, &hi)?;
                merged = apply_operation(
                    &merged,
                    &Operation::Insert {
                        destination,
                        value: value.clone(),
                    },
                )?;
            }
            Operation::Delete { target } => {
                // A baseline ancestor deletion also removes a concurrently
                // escaped descendant; the move cannot resurrect that subtree.
                let mut removed = low_before.get(*target)?.ids();
                if let Some(original) = base.find(*target) {
                    removed.extend(
                        original
                            .ids()
                            .into_iter()
                            .filter(|id| low_value.find(*id).is_none()),
                    );
                }
                merged = remove_present(merged, &removed)?;
            }
            Operation::Set { target, value } => {
                if merged.find(*target).is_some() && !hi.set.contains(target) {
                    let mut removed = low_before.get(*target)?.ids();
                    if let Some(original) = base.find(*target) {
                        removed.extend(
                            original
                                .ids()
                                .into_iter()
                                .filter(|id| low_value.find(*id).is_none()),
                        );
                    }
                    removed.remove(target);
                    for id in value.ids() {
                        removed.remove(&id);
                    }
                    merged = remove_present(merged, &removed)?;
                    let replacement = preserve_incoming(&merged.get(*target)?, value)?;
                    merged = apply_operation(
                        &merged,
                        &Operation::Set {
                            target: *target,
                            value: replacement,
                        },
                    )?;
                }
            }
            Operation::Move {
                target,
                destination,
            } => {
                if !hi.moved.contains_key(target)
                    && merged.find(*target).is_some()
                    && !(low_value.find(*target).is_none()
                        && merged.find(destination.parent).is_none())
                    && !(lo.moved.get(target).is_some_and(|last| *last > index)
                        && (merged.find(destination.parent).is_none()
                            || merged
                                .find(*target)
                                .is_some_and(|source| source.find(destination.parent).is_some())))
                {
                    let destination =
                        rebase_destination(&low_before, &merged, destination, Some(*target), &hi)?;
                    merged = apply_operation(
                        &merged,
                        &Operation::Move {
                            target: *target,
                            destination,
                        },
                    )?;
                }
            }
            Operation::Text { .. } | Operation::Add { .. } | Operation::RichText { .. } => {}
        }
        low_before = apply_operation(&low_before, operation)?;
    }
    // Content targets are stable identities. Moving an ancestor cannot redirect
    // an edit to the element that happens to occupy its old path.
    let mut ids = BTreeSet::new();
    ids.extend(lo.text.keys().copied());
    ids.extend(lo.added.keys().copied());
    ids.extend(lo.set.iter().copied());
    ids.extend(lo.rich.keys().copied());
    for id in ids {
        let Some(current) = merged.find(id) else {
            continue;
        };
        let Some(low_node) = low_value.find(id) else {
            continue;
        };
        if hi.set.contains(&id) {
            continue;
        }
        if lo.set.contains(&id) || base.find(id).is_none() {
            if !matches!(low_node.body(), Body::List(_) | Body::Map(_)) {
                merged = merged.replace_at(id, low_node)?;
            }
            continue;
        }
        if let Some(low_text) = lo.text.get(&id) {
            let Body::Text(text) = current.body() else {
                continue;
            };
            let high_text = hi.text.get(&id).cloned().unwrap_or_default();
            let (_, low_after_high) =
                crate::sequence::op::transform(&high_text, low_text, TieBreak::LeftFirst)
                    .map_err(algebra_error)?;
            let result = crate::sequence::op::apply(
                &crate::sequence::value::Value::text(text.clone()),
                &low_after_high,
            )?;
            merged = merged.replace_at(
                id,
                &Value::trusted(id, Body::Text(result.as_text().unwrap().as_str().into())),
            )?;
        } else if let Some(delta) = lo.added.get(&id) {
            let Body::Int(number) = current.body() else {
                continue;
            };
            let number = i64::try_from(i128::from(*number) + delta).map_err(|_| {
                Error::new(ErrorCode::IntegerOverflow, "concurrent additions overflow")
            })?;
            merged = merged.replace_at(id, &Value::trusted(id, Body::Int(number)))?;
        } else if let Some(low_rich) = lo.rich.get(&id) {
            let Body::RichText(spans) = current.body() else {
                continue;
            };
            let high_rich = hi.rich.get(&id).cloned().unwrap_or_default();
            let (_, low_after_high) =
                crate::sequence::op::transform(&high_rich, low_rich, TieBreak::LeftFirst)
                    .map_err(algebra_error)?;
            let result =
                crate::sequence::op::apply(&super::rich::value_to_old(spans)?, &low_after_high)?;
            merged = merged.replace_at(
                id,
                &Value::trusted(id, Body::RichText(super::rich::value_from_old(&result)?)),
            )?;
        }
    }
    merged.validate()?;
    Ok(merged)
}

fn remove_present(mut value: Value, ids: &BTreeSet<ElementId>) -> Result<Value> {
    for id in ids {
        if value.find(*id).is_some() {
            value = apply_operation(&value, &Operation::Delete { target: *id })?;
        }
    }
    Ok(value)
}

fn rebase_destination(
    before: &Value,
    after: &Value,
    destination: &Destination,
    source: Option<ElementId>,
    high: &Intents,
) -> Result<Destination> {
    let parent = after
        .find(destination.parent)
        .ok_or_else(|| structural("move or insertion destination parent no longer exists"))?;
    let slot = match (&destination.slot, parent.body()) {
        (Segment::Key(key), Body::Map(_)) => Segment::Key(key.clone()),
        (Segment::Index(index), Body::List(current)) => {
            let original = before.get(destination.parent)?;
            let Body::List(original) = original.body() else {
                return Err(structural("destination parent kind changed"));
            };
            let original: Vec<_> = original.iter().filter(|v| Some(v.id()) != source).collect();
            let current: Vec<_> = current.iter().filter(|v| Some(v.id()) != source).collect();
            let mut position = current.len();
            // Retain a gap's surviving right neighbour. Explicitly moved
            // neighbours no longer anchor their former gap.
            for anchor in original.iter().skip(*index) {
                if high.moved.contains_key(&anchor.id()) {
                    continue;
                }
                if let Some(found) = current.iter().position(|v| v.id() == anchor.id()) {
                    position = found;
                    break;
                }
            }
            Segment::Index(position)
        }
        _ => return Err(structural("destination parent kind changed")),
    };
    Ok(Destination {
        parent: destination.parent,
        slot,
    })
}

/// Builds an executable identity-aware delta. Reordering is expressed with
/// native moves, including temporary parking when Map keys form a permutation.
fn difference(
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
fn algebra_error(error: impl std::fmt::Display) -> Error {
    Error::new(ErrorCode::IncompatibleChange, error.to_string())
}
