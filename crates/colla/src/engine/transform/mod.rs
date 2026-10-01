use super::{
    change::{apply_operation, destination_of, structural},
    Body, Change, Destination, ElementId, Error, ErrorCode, Operation, Result, Segment, Value,
};
use crate::sequence::change::TieBreak;
mod delta;

use delta::difference;
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

fn algebra_error(error: impl std::fmt::Display) -> Error {
    Error::new(ErrorCode::IncompatibleChange, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::apply;

    fn key(name: &str) -> Segment {
        Segment::Key(name.into())
    }
    fn map(entries: Vec<(&str, Value)>) -> Value {
        Value::map(entries.into_iter().map(|(k, v)| (k.to_string(), v))).unwrap()
    }
    fn change(operations: Vec<Operation>) -> Change {
        Change::new(operations).unwrap()
    }
    fn converges(base: &Value, left: &Change, right: &Change, priority: Priority) -> Value {
        let (left_after, right_after) = transform(base, left, right, priority).unwrap();
        let merged = apply(&apply(base, right).unwrap(), &left_after).unwrap();
        assert_eq!(
            merged,
            apply(&apply(base, left).unwrap(), &right_after).unwrap()
        );
        merged
    }

    #[test]
    fn identical_changes_take_the_fast_path() {
        let counter = Value::int(1);
        let base = map(vec![("n", counter.clone())]);
        let set = change(vec![Operation::Set {
            target: counter.id(),
            value: Value::with_id(counter.id(), Body::Int(5)).unwrap(),
        }]);
        let (left, right) = transform(&base, &set, &set, Priority::Left).unwrap();
        assert!(left.is_noop() && right.is_noop());
    }

    #[test]
    fn move_into_concurrently_deleted_parent_conflicts() {
        let item = Value::int(1);
        let target = map(vec![]);
        let base = map(vec![
            ("items", Value::list(vec![item.clone()]).unwrap()),
            ("target", target.clone()),
        ]);
        let moved = change(vec![Operation::Move {
            target: item.id(),
            destination: Destination {
                parent: target.id(),
                slot: key("k"),
            },
        }]);
        let deleted = change(vec![Operation::Delete {
            target: target.id(),
        }]);
        for priority in [Priority::Left, Priority::Right] {
            let error = transform(&base, &moved, &deleted, priority).unwrap_err();
            assert_eq!(error.code, ErrorCode::StructuralConflict);
        }
    }

    #[test]
    fn set_competing_with_incoming_map_key_conflicts() {
        let inner = map(vec![]);
        let base = map(vec![("m", inner.clone())]);
        let set = change(vec![Operation::Set {
            target: inner.id(),
            value: Value::with_id(
                inner.id(),
                Body::Map([("k".to_string(), Value::int(1))].into()),
            )
            .unwrap(),
        }]);
        let insert = change(vec![Operation::Insert {
            destination: Destination {
                parent: inner.id(),
                slot: key("k"),
            },
            value: Value::int(2),
        }]);
        let error = transform(&base, &set, &insert, Priority::Left).unwrap_err();
        assert_eq!(error.code, ErrorCode::StructuralConflict);
    }

    #[test]
    fn set_preserves_concurrently_inserted_list_items() {
        let list = Value::list(vec![Value::int(1)]).unwrap();
        let base = map(vec![("l", list.clone())]);
        let set = change(vec![Operation::Set {
            target: list.id(),
            value: Value::with_id(list.id(), Body::List(vec![Value::int(9)])).unwrap(),
        }]);
        let incoming = Value::int(2);
        let insert = change(vec![Operation::Insert {
            destination: Destination {
                parent: list.id(),
                slot: Segment::Index(1),
            },
            value: incoming.clone(),
        }]);
        let merged = converges(&base, &set, &insert, Priority::Left);
        assert!(merged.find(incoming.id()).is_some());
        assert!(merged.find(list.id()).is_some());
    }

    #[test]
    fn competing_moves_follow_priority_and_keep_identity() {
        let item = Value::int(1);
        let a = Value::list(vec![]).unwrap();
        let b = Value::list(vec![]).unwrap();
        let base = map(vec![
            ("items", Value::list(vec![item.clone()]).unwrap()),
            ("a", a.clone()),
            ("b", b.clone()),
        ]);
        let move_to = |parent: ElementId| {
            change(vec![Operation::Move {
                target: item.id(),
                destination: Destination {
                    parent,
                    slot: Segment::Index(0),
                },
            }])
        };
        let (left, right) = (move_to(a.id()), move_to(b.id()));
        for (priority, winner) in [(Priority::Left, a.id()), (Priority::Right, b.id())] {
            let merged = converges(&base, &left, &right, priority);
            assert_eq!(destination_of(&merged, item.id()).unwrap().parent, winner);
        }
    }
}
