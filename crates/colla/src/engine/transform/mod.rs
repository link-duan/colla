use super::{
    change::{algebra_error, apply_operation, type_error},
    Body, Change, Operation, Result, Segment, Value,
};
use crate::sequence::change::TieBreak;
use list::{map_index, ListEdit};
mod list;

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
    let mut rights = right.operations().to_vec();
    let mut lefts = Vec::new();
    let mut before_left = State::new(base);
    let left_first = priority == Priority::Left;
    for operation in left.operations() {
        // Transform one left step across every right step, keeping the base
        // that both current versions apply to.
        let mut current = Some(operation.clone());
        let mut state = before_left.clone();
        let mut transformed = Vec::with_capacity(rights.len());
        for other in &rights {
            if let Some(mine) = &current {
                let after = rebase(&mut state, mine, other, left_first)?;
                transformed.extend(rebase(&mut state, other, mine, !left_first)?);
                current = after;
            } else {
                transformed.push(other.clone());
            }
            state.push(other);
        }
        lefts.extend(current);
        before_left.push(operation);
        rights = transformed;
    }
    Ok((
        Change::new(lefts)?.normalized(&right_value)?.0,
        Change::new(rights)?.normalized(&left_value)?.0,
    ))
}

// The content both current operations apply to, materialized only when a List
// length is needed: most pairs transform from their paths alone.
#[derive(Clone)]
struct State<'a> {
    value: Value,
    pending: Vec<&'a Operation>,
}
impl<'a> State<'a> {
    fn new(value: &Value) -> Self {
        Self {
            value: value.clone(),
            pending: Vec::new(),
        }
    }
    fn push(&mut self, operation: &'a Operation) {
        self.pending.push(operation);
    }
    fn get(&mut self) -> Result<&Value> {
        for operation in self.pending.drain(..) {
            self.value = apply_operation(&self.value, operation)?;
        }
        Ok(&self.value)
    }
}

fn list_edit(operation: &Operation) -> Option<(&[Segment], ListEdit)> {
    match operation {
        Operation::Insert { path, .. } => match path.split_last() {
            Some((Segment::Index(index), list)) => Some((list, ListEdit::Insert(*index))),
            _ => unreachable!("Change construction restricts Insert to List indexes"),
        },
        Operation::Delete { path } => match path.split_last() {
            Some((Segment::Index(index), list)) => Some((list, ListEdit::Delete(*index))),
            _ => None,
        },
        Operation::ListMove { path, from, to } => Some((
            path,
            ListEdit::Move {
                from: *from,
                to: *to,
            },
        )),
        _ => None,
    }
}
fn with_list_edit(operation: &Operation, list: &[Segment], edit: ListEdit) -> Operation {
    let at = |index| [list, &[Segment::Index(index)]].concat();
    match (operation, edit) {
        (Operation::Insert { value, .. }, ListEdit::Insert(index)) => Operation::Insert {
            path: at(index),
            value: value.clone(),
        },
        (Operation::Delete { .. }, ListEdit::Delete(index)) => {
            Operation::Delete { path: at(index) }
        }
        (Operation::ListMove { .. }, ListEdit::Move { from, to }) => Operation::ListMove {
            path: list.to_vec(),
            from,
            to,
        },
        _ => unreachable!("List rebase preserves the edit kind"),
    }
}
// The element-addressed prefix: an Insert's List index is a gap, not an element.
fn element_path(operation: &Operation) -> &[Segment] {
    match operation {
        Operation::Insert { path, .. } => &path[..path.len() - 1],
        other => other.path(),
    }
}

/// Returns `mine` transformed to apply after `other`, both relative to `state`.
fn rebase(
    state: &mut State,
    mine: &Operation,
    other: &Operation,
    mine_first: bool,
) -> Result<Option<Operation>> {
    if let (Some((list, edit)), Some((other_list, other_edit))) =
        (list_edit(mine), list_edit(other))
    {
        if list == other_list {
            let Body::List(items) = state.get()?.get(list)?.body() else {
                return Err(type_error("expected List"));
            };
            return Ok(list::rebase(items.len(), edit, other_edit, mine_first)
                .map(|edit| with_list_edit(mine, list, edit)));
        }
    }
    match other {
        Operation::Delete { path: removed } | Operation::Set { path: removed, .. } => {
            let deleting = matches!(other, Operation::Delete { .. });
            let target = element_path(mine);
            if target.starts_with(removed) {
                // Content inside a deleted or replaced element is discarded.
                // On the same element Delete wins and competing Sets, including
                // Sets creating one Map member, follow priority.
                let same = target.len() == removed.len()
                    && !matches!(mine, Operation::Insert { .. } | Operation::ListMove { .. });
                return Ok(match mine {
                    Operation::Delete { .. } if same && !deleting => Some(mine.clone()),
                    Operation::Set { .. } if same && !deleting && mine_first => Some(mine.clone()),
                    _ => None,
                });
            }
        }
        Operation::Text { path, change } => {
            if let Operation::Text {
                path: mine_path,
                change: mine_change,
            } = mine
            {
                if mine_path == path {
                    let (change, _) = crate::sequence::op::transform(
                        &mine_change.clone().into(),
                        &change.clone().into(),
                        tie(mine_first),
                    )
                    .map_err(algebra_error)?;
                    return Ok(Some(Operation::Text {
                        path: path.clone(),
                        change: change.as_text().cloned().unwrap_or_default(),
                    }));
                }
            }
        }
        Operation::RichText { path, operations } => {
            if let Operation::RichText {
                path: mine_path,
                operations: mine_operations,
            } = mine
            {
                if mine_path == path {
                    let (change, _) = crate::sequence::op::transform(
                        &super::rich::to_sequence_change(mine_operations)?,
                        &super::rich::to_sequence_change(operations)?,
                        tie(mine_first),
                    )
                    .map_err(algebra_error)?;
                    return Ok(Some(Operation::RichText {
                        path: path.clone(),
                        operations: super::rich::from_sequence_change(&change)?,
                    }));
                }
            }
        }
        Operation::Insert { .. } | Operation::ListMove { .. } | Operation::Add { .. } => {}
    }
    let Some((list, edit)) = list_edit(other) else {
        return Ok(Some(mine.clone()));
    };
    // An element-addressed segment inside the edited List follows its element.
    let target = element_path(mine);
    if target.len() <= list.len() || !target.starts_with(list) {
        return Ok(Some(mine.clone()));
    }
    let Segment::Index(index) = target[list.len()] else {
        return Err(type_error("path segment does not match container"));
    };
    let Some(index) = map_index(index, edit) else {
        return Ok(None);
    };
    let mut mine = mine.clone();
    let path = match &mut mine {
        Operation::Insert { path, .. }
        | Operation::Delete { path }
        | Operation::Set { path, .. }
        | Operation::ListMove { path, .. }
        | Operation::Text { path, .. }
        | Operation::Add { path, .. }
        | Operation::RichText { path, .. } => path,
    };
    path[list.len()] = Segment::Index(index);
    Ok(Some(mine))
}

fn tie(mine_first: bool) -> TieBreak {
    if mine_first {
        TieBreak::LeftFirst
    } else {
        TieBreak::RightFirst
    }
}
