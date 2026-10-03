//! Transformation of two structural edits on the same List.
//!
//! Each edit is described by the base elements it removes or moves and by the
//! element it places before an anchor: a base element or the end. Anchors that
//! the other edit removes or moves are replaced by the next stable element.
//! Both sides build the same merged order, and each transformed edit is derived
//! as the single step from the other side's result to that order.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ListEdit {
    Insert(usize),
    Delete(usize),
    Move { from: usize, to: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    Base(usize),
    Mine,
    Other,
}

struct Placement {
    token: Token,
    // None anchors at the end of the List.
    anchor: Option<usize>,
}

fn placement(edit: ListEdit, token: Token, len: usize) -> Option<Placement> {
    match edit {
        ListEdit::Insert(gap) => Some(Placement {
            token,
            anchor: (gap < len).then_some(gap),
        }),
        ListEdit::Move { from, to } => Some(Placement {
            token: Token::Base(from),
            anchor: (to < len - 1).then(|| to + usize::from(to >= from)),
        }),
        ListEdit::Delete(_) => None,
    }
}
fn deleted(edit: ListEdit) -> Option<usize> {
    match edit {
        ListEdit::Delete(index) => Some(index),
        _ => None,
    }
}
fn moved(edit: ListEdit) -> Option<usize> {
    match edit {
        ListEdit::Move { from, .. } => Some(from),
        _ => None,
    }
}

// Placements are ordered by precedence: earlier entries win shared anchors.
fn order(len: usize, edits: &[(ListEdit, Token)]) -> Vec<Token> {
    let removed = |index| edits.iter().any(|(edit, _)| deleted(*edit) == Some(index));
    let mut placements = Vec::new();
    for (position, (edit, token)) in edits.iter().enumerate() {
        let Some(placement) = placement(*edit, *token, len) else {
            continue;
        };
        if let Token::Base(index) = placement.token {
            // Deletion wins over movement; competing moves keep the first.
            if removed(index)
                || edits[..position]
                    .iter()
                    .any(|(edit, _)| moved(*edit) == Some(index))
            {
                continue;
            }
        }
        placements.push(placement);
    }
    let stable =
        |index| !removed(index) && !edits.iter().any(|(edit, _)| moved(*edit) == Some(index));
    let anchor = |anchor: Option<usize>| anchor.and_then(|start| (start..len).find(|i| stable(*i)));
    let mut result = Vec::with_capacity(len + 2);
    for index in 0..len {
        for placement in &placements {
            if anchor(placement.anchor) == Some(index) {
                result.push(placement.token);
            }
        }
        if stable(index) {
            result.push(Token::Base(index));
        }
    }
    for placement in &placements {
        if anchor(placement.anchor).is_none() {
            result.push(placement.token);
        }
    }
    result
}

/// Returns `mine` transformed to apply after `other`, both relative to a List of `len`.
pub(super) fn rebase(
    len: usize,
    mine: ListEdit,
    other: ListEdit,
    mine_first: bool,
) -> Option<ListEdit> {
    let merged = if mine_first {
        order(len, &[(mine, Token::Mine), (other, Token::Other)])
    } else {
        order(len, &[(other, Token::Other), (mine, Token::Mine)])
    };
    let theirs = order(len, &[(other, Token::Other)]);
    let position = |order: &[Token], token| order.iter().position(|t| *t == token);
    match mine {
        ListEdit::Insert(_) => position(&merged, Token::Mine).map(ListEdit::Insert),
        ListEdit::Delete(index) => position(&theirs, Token::Base(index)).map(ListEdit::Delete),
        ListEdit::Move { from, .. } => {
            let to = position(&merged, Token::Base(from))?;
            if moved(other) == Some(from) && !mine_first {
                return None;
            }
            Some(ListEdit::Move {
                from: position(&theirs, Token::Base(from))?,
                to,
            })
        }
    }
}

/// Maps an existing element index through one List edit; None if it was deleted.
pub(super) fn map_index(index: usize, edit: ListEdit) -> Option<usize> {
    match edit {
        ListEdit::Insert(gap) => Some(index + usize::from(index >= gap)),
        ListEdit::Delete(deleted) => match index.cmp(&deleted) {
            std::cmp::Ordering::Less => Some(index),
            std::cmp::Ordering::Equal => None,
            std::cmp::Ordering::Greater => Some(index - 1),
        },
        ListEdit::Move { from, to } => {
            if index == from {
                return Some(to);
            }
            let index = index - usize::from(index > from);
            Some(index + usize::from(index >= to))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(list: &[u32], edit: ListEdit, inserted: u32) -> Vec<u32> {
        let mut list = list.to_vec();
        match edit {
            ListEdit::Insert(index) => list.insert(index, inserted),
            ListEdit::Delete(index) => {
                list.remove(index);
            }
            ListEdit::Move { from, to } => {
                let item = list.remove(from);
                list.insert(to, item);
            }
        }
        list
    }
    fn edits(len: usize) -> Vec<ListEdit> {
        let mut out: Vec<_> = (0..=len).map(ListEdit::Insert).collect();
        out.extend((0..len).map(ListEdit::Delete));
        for from in 0..len {
            for to in 0..len {
                out.push(ListEdit::Move { from, to });
            }
        }
        out
    }
    fn after(list: &[u32], edit: Option<ListEdit>, inserted: u32) -> Vec<u32> {
        edit.map_or_else(|| list.to_vec(), |edit| apply(list, edit, inserted))
    }

    #[test]
    fn every_pair_converges_with_either_priority() {
        for len in 0..5usize {
            let base: Vec<u32> = (0..len as u32).collect();
            for a in edits(len) {
                for b in edits(len) {
                    for a_first in [true, false] {
                        let a2 = rebase(len, a, b, a_first);
                        let b2 = rebase(len, b, a, !a_first);
                        let left = after(&apply(&base, a, 100), b2, 200);
                        let right = after(&apply(&base, b, 200), a2, 100);
                        assert_eq!(left, right, "{a:?} {b:?} a_first={a_first}");
                    }
                }
            }
        }
    }

    #[test]
    fn element_mapping_matches_application() {
        for len in 1..5usize {
            let base: Vec<u32> = (0..len as u32).collect();
            for edit in edits(len) {
                let result = apply(&base, edit, 100);
                for (index, item) in base.iter().enumerate() {
                    match map_index(index, edit) {
                        Some(mapped) => assert_eq!(result[mapped], *item),
                        None => assert!(!result.contains(item)),
                    }
                }
            }
        }
    }
}
