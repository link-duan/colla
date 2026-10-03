use super::{
    apply, change::type_error, compose, invert, Body, Change, Error, ErrorCode, Operation, Result,
    RichOp, Segment, Value,
};
use crate::sequence::change::{TextChange, TextOp};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The source of a committed visible edit.
pub enum Origin {
    /// An ordinary local transaction.
    Local,
    /// A remotely committed content change.
    Remote,
    /// An inverse submitted as a new local edit.
    Undo,
    /// A redo submitted as a new local edit.
    Redo,
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// Complete immutable evidence of an atomic content commit.
pub struct EditResult {
    /// Immutable content immediately before this commit.
    pub before: Value,
    /// Immutable content immediately after this commit.
    pub after: Value,
    /// Path-addressed operations, sequentially replayable from `before`.
    pub change: Change,
    /// Reverse Change restoring `before` from `after`.
    pub inverse: Change,
    /// Local content version after this commit.
    pub version: u64,
    /// Source of the committed edit.
    pub origin: Origin,
}

#[derive(Clone)]
/// Shared visible content and atomic editing runtime.
pub struct Document {
    pub(crate) shared: Rc<Shared>,
}
pub(crate) struct Shared {
    pub(crate) phase: Cell<Phase>,
    pub(crate) state: RefCell<State>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Idle,
    Editing,
    Closed,
}
#[derive(Clone)]
pub(crate) struct State {
    pub(crate) value: Value,
    pub(crate) version: u64,
    pub(crate) history: Option<super::history::HistoryData>,
    pub(crate) history_epoch: u64,
    pub(crate) sync: Option<super::sync::SessionData>,
}

impl Document {
    /// Creates a runtime from validated initial content.
    pub fn create(value: Value) -> Result<Self> {
        value.validate()?;
        Ok(Self {
            shared: Rc::new(Shared {
                phase: Cell::new(Phase::Idle),
                state: RefCell::new(State {
                    value,
                    version: 0,
                    history: None,
                    history_epoch: 0,
                    sync: None,
                }),
            }),
        })
    }
    /// Returns the immutable content snapshot for this runtime.
    pub fn snapshot(&self) -> Result<Value> {
        self.readable()?;
        Ok(self.shared.state.borrow().value.clone())
    }
    /// Returns the local visible-content version, distinct from server revision.
    pub fn version(&self) -> Result<u64> {
        self.readable()?;
        Ok(self.shared.state.borrow().version)
    }
    /// Looks up a Path; missing or incompatible locations return an error.
    pub fn get(&self, path: &[Segment]) -> Result<Value> {
        self.snapshot()?.get(path).cloned()
    }
    /// Returns the kind at a Path, requiring the target to exist.
    pub fn kind(&self, path: &[Segment]) -> Result<&'static str> {
        Ok(self.get(path)?.kind())
    }
    /// Returns whether the Path resolves in this content.
    pub fn has(&self, path: &[Segment]) -> Result<bool> {
        Ok(self.snapshot()?.has(path))
    }
    /// Runs one synchronous atomic editing scope; a normalized Noop returns None.
    pub fn edit(
        &self,
        callback: impl FnOnce(&mut Transaction) -> Result<()>,
    ) -> Result<Option<EditResult>> {
        self.edit_group(None, callback)
    }
    /// Runs an atomic edit and joins consecutive local edits with the same explicit group.
    pub fn edit_group(
        &self,
        group: Option<String>,
        callback: impl FnOnce(&mut Transaction) -> Result<()>,
    ) -> Result<Option<EditResult>> {
        let mut transaction = self.begin()?;
        transaction.group = group;
        callback(&mut transaction)?;
        transaction.commit()
    }
    /// Applies a path-addressed Change atomically against the current content.
    pub fn apply(&self, change: &Change) -> Result<Option<EditResult>> {
        self.edit(|tx| tx.apply(change))
    }
    /// Closes this runtime idempotently; previously returned immutable content remains valid.
    pub fn close(&self) -> Result<()> {
        match self.shared.phase.get() {
            Phase::Editing => Err(invalid_state("cannot close during an edit")),
            Phase::Idle | Phase::Closed => {
                self.shared.phase.set(Phase::Closed);
                Ok(())
            }
        }
    }
    pub(crate) fn begin(&self) -> Result<Transaction> {
        self.idle()?;
        let value = self.shared.state.borrow().value.clone();
        self.shared.phase.set(Phase::Editing);
        Ok(Transaction {
            document: self.clone(),
            before: value.clone(),
            value,
            change: Change::noop(),
            active: true,
            group: None,
        })
    }
    pub(crate) fn readable(&self) -> Result<()> {
        if self.shared.phase.get() == Phase::Closed {
            return Err(invalid_state("document is closed"));
        }
        Ok(())
    }
    pub(crate) fn idle(&self) -> Result<()> {
        if self.shared.phase.get() != Phase::Idle {
            return Err(invalid_state("document is not idle"));
        }
        Ok(())
    }
}

/// A scope owns its working tree. Dropping it rolls back.
pub struct Transaction {
    document: Document,
    before: Value,
    value: Value,
    change: Change,
    active: bool,
    pub(crate) group: Option<String>,
}
impl Drop for Transaction {
    fn drop(&mut self) {
        if self.active {
            self.document.shared.phase.set(Phase::Idle);
        }
    }
}
impl Transaction {
    fn check(&self) -> Result<()> {
        if !self.active {
            return Err(invalid_state("transaction scope has ended"));
        }
        Ok(())
    }
    /// Returns the immutable content snapshot for this runtime.
    pub fn snapshot(&self) -> Result<Value> {
        self.check()?;
        Ok(self.value.clone())
    }
    /// Looks up a Path; missing or incompatible locations return an error.
    pub fn get(&self, path: &[Segment]) -> Result<Value> {
        self.check()?;
        self.value.get(path).cloned()
    }
    /// Applies a path-addressed Change atomically against the current content.
    pub fn apply(&mut self, change: &Change) -> Result<()> {
        self.check()?;
        let value = apply(&self.value, change)?;
        let change = compose(&self.before, &self.change, change)?;
        self.value = value;
        self.change = change;
        Ok(())
    }
    fn run(&mut self, operations: impl IntoIterator<Item = Operation>) -> Result<()> {
        self.apply(&Change::new(operations)?)
    }
    /// Replaces an existing element, or inserts a missing Map member.
    pub fn set(&mut self, path: &[Segment], value: Value) -> Result<()> {
        self.check()?;
        let path = path.to_vec();
        let operation = match self.value.get(&path) {
            Ok(_) => Operation::Set { path, value },
            Err(error) => {
                let Some((Segment::Key(_), parent)) = path.split_last() else {
                    return Err(error);
                };
                if !matches!(self.value.get(parent)?.body(), Body::Map(_)) {
                    return Err(type_error("set requires an existing Map parent"));
                }
                Operation::Insert { path, value }
            }
        };
        self.run([operation])
    }
    /// Deletes an existing non-root element and its owned subtree.
    pub fn delete(&mut self, path: &[Segment]) -> Result<()> {
        self.run([Operation::Delete {
            path: path.to_vec(),
        }])
    }
    /// Moves one element within a List; `to` is interpreted after removal.
    pub fn list_move(&mut self, path: &[Segment], from: usize, to: usize) -> Result<()> {
        self.run([Operation::ListMove {
            path: path.to_vec(),
            from,
            to,
        }])
    }
    /// Inserts a copy of the source subtree at a vacant Map key or List position.
    pub fn copy(&mut self, source: &[Segment], destination: &[Segment]) -> Result<()> {
        let value = self.get(source)?;
        self.run([Operation::Insert {
            path: destination.to_vec(),
            value,
        }])
    }
    /// Adds a checked i64 delta to an Int.
    pub fn increment(&mut self, path: &[Segment], delta: i64) -> Result<()> {
        self.run([Operation::Add {
            path: path.to_vec(),
            delta,
        }])
    }
    /// Replaces a List range; zero count inserts and an empty input deletes.
    pub fn list_replace(
        &mut self,
        path: &[Segment],
        index: usize,
        count: usize,
        values: Vec<Value>,
    ) -> Result<()> {
        let len = match self.get(path)?.body() {
            Body::List(list) => list.len(),
            _ => return Err(type_error("expected List")),
        };
        checked_end(index, count, len)?;
        let at = |index: usize| [path, &[Segment::Index(index)]].concat();
        let mut operations: Vec<_> = (0..count)
            .map(|_| Operation::Delete { path: at(index) })
            .collect();
        for (offset, value) in values.into_iter().enumerate() {
            let index = index
                .checked_add(offset)
                .ok_or_else(|| Error::new(ErrorCode::LimitExceeded, "List index overflow"))?;
            operations.push(Operation::Insert {
                path: at(index),
                value,
            });
        }
        self.run(operations)
    }
    /// Replaces a Text range using Unicode scalar coordinates in current working content.
    pub fn text_replace(
        &mut self,
        path: &[Segment],
        index: usize,
        count: usize,
        text: &str,
    ) -> Result<()> {
        let len = match self.get(path)?.body() {
            Body::Text(text) => text.chars().count(),
            _ => return Err(type_error("expected Text")),
        };
        checked_end(index, count, len)?;
        self.run([Operation::Text {
            path: path.to_vec(),
            change: TextChange::from_ops([
                TextOp::Retain(index),
                TextOp::Delete(count),
                TextOp::Insert(text.into()),
            ])?,
        }])
    }
    /// Applies scalar RichText operations to the current working content.
    pub fn rich_text_edit(&mut self, path: &[Segment], operations: Vec<RichOp>) -> Result<()> {
        self.run([Operation::RichText {
            path: path.to_vec(),
            operations,
        }])
    }
    /// Converts a working Text/RichText offset, rejecting surrogate splits and out-of-bounds positions.
    pub fn utf16_to_scalar(&self, path: &[Segment], position: usize) -> Result<usize> {
        let value = self.get(path)?;
        let result = match value.body() {
            Body::Text(text) => {
                crate::sequence::value::Text::new(text.clone()).utf16_to_code_point(position)
            }
            Body::RichText(spans) => super::rich::to_sequence_value(spans)?
                .as_rich_text()
                .unwrap()
                .utf16_to_code_point(position),
            _ => return Err(type_error("UTF-16 coordinates require Text or RichText")),
        };
        result.map_err(|error| match error {
            crate::sequence::error::Utf16PositionError::InvalidUtf16Boundary { .. } => {
                Error::new(ErrorCode::InvalidUtf16Boundary, error.to_string())
            }
            _ => Error::new(ErrorCode::OutOfBounds, error.to_string()),
        })
    }
    pub(crate) fn commit(&mut self) -> Result<Option<EditResult>> {
        self.check()?;
        let (change, after) = self.change.normalized(&self.before)?;
        if change.is_noop() {
            self.finish();
            return Ok(None);
        }
        debug_assert_eq!(after, self.value);
        let (next, result) = self.document.shared.state.borrow().prepare(
            &change,
            Origin::Local,
            self.group.clone(),
        )?;
        *self.document.shared.state.borrow_mut() = next;
        self.finish();
        Ok(Some(result))
    }
    pub(crate) fn finish(&mut self) {
        self.active = false;
        self.document.shared.phase.set(Phase::Idle);
    }
}
impl State {
    pub(crate) fn history(&self) -> Result<&super::history::HistoryData> {
        self.history
            .as_ref()
            .ok_or_else(|| invalid_state("History is closed"))
    }
    pub(crate) fn history_mut(&mut self) -> Result<&mut super::history::HistoryData> {
        self.history
            .as_mut()
            .ok_or_else(|| invalid_state("History is closed"))
    }
    pub(crate) fn session(&self) -> Result<&super::sync::SessionData> {
        self.sync
            .as_ref()
            .ok_or_else(|| invalid_state("SyncSession is not attached"))
    }
    pub(crate) fn session_mut(&mut self) -> Result<&mut super::sync::SessionData> {
        self.sync
            .as_mut()
            .ok_or_else(|| invalid_state("SyncSession is not attached"))
    }
    pub(crate) fn prepare(
        &self,
        change: &Change,
        origin: Origin,
        group: Option<String>,
    ) -> Result<(Self, EditResult)> {
        let after = apply(&self.value, change)?;
        let version = self
            .version
            .checked_add(1)
            .ok_or_else(|| Error::new(ErrorCode::LimitExceeded, "local version exhausted"))?;
        let result = EditResult {
            before: self.value.clone(),
            after: after.clone(),
            inverse: invert(&self.value, change)?,
            change: change.clone(),
            version,
            origin,
        };
        let mut next = self.clone();
        if let Some(history) = &mut next.history {
            match origin {
                Origin::Local => history.record(&result, group)?,
                Origin::Remote => history.rebase(&self.value, change)?,
                Origin::Undo | Origin::Redo => history.group = None,
            }
        }
        if origin != Origin::Remote {
            if let Some(sync) = &mut next.sync {
                sync.enqueue(change)?;
            }
        }
        next.value = after;
        next.version = version;
        Ok((next, result))
    }
}
pub(crate) fn invalid_state(reason: &str) -> Error {
    Error::new(ErrorCode::InvalidState, reason)
}
fn checked_end(index: usize, count: usize, len: usize) -> Result<usize> {
    index
        .checked_add(count)
        .filter(|end| *end <= len)
        .ok_or_else(|| Error::new(ErrorCode::OutOfBounds, "sequence range is out of bounds"))
}
