use super::{
    apply, compose, invert, Body, Change, Destination, ElementId, Error, ErrorCode, Location,
    Operation, Result, RichOp, Segment, Value,
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
    /// Immutable identity-addressed or scalar change content.
    pub change: Change,
    /// Reverse Change preserving all original owning identities.
    pub inverse: Change,
    /// Sequentially replayable scalar operations, retaining native Move source IDs.
    pub edit_steps: Vec<Operation>,
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
    /// Creates a runtime from validated initial content, retaining its owning identities.
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
    /// Looks up a Path or stable ID; missing or incompatible locations return an error.
    pub fn get(&self, location: impl Into<Location>) -> Result<Value> {
        self.snapshot()?.get(location)
    }
    /// Returns the kind at a Path or ID, requiring the target to exist.
    pub fn kind(&self, location: impl Into<Location>) -> Result<&'static str> {
        Ok(self.get(location)?.kind())
    }
    /// Returns whether the Path or stable ID resolves in this content.
    pub fn has(&self, location: impl Into<Location>) -> Result<bool> {
        Ok(self.snapshot()?.has(location))
    }
    /// Returns the stable owning ID at an existing Path.
    pub fn id_at(&self, path: super::Path) -> Result<ElementId> {
        self.snapshot()?.id_at(path)
    }
    /// Derives the current Path from parent links, or returns None for an absent ID.
    pub fn path_of(&self, id: ElementId) -> Result<Option<super::Path>> {
        Ok(self.snapshot()?.path_of(id))
    }
    /// Resolves one weak Ref hop within this snapshot; dangling targets return None.
    pub fn resolve(&self, reference: super::Ref) -> Result<Option<Value>> {
        Ok(self.snapshot()?.resolve(reference))
    }
    /// Returns referring element IDs, including Ref values within atomic embeds.
    pub fn references_to(&self, id: ElementId) -> Result<Vec<ElementId>> {
        Ok(self.snapshot()?.references_to(id))
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
    /// Applies an identity-addressed Change atomically against the current content.
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

/// A scope owns its working tree. Dropping it rolls back without rewinding IDs.
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
    /// Looks up a Path or stable ID; missing or incompatible locations return an error.
    pub fn get(&self, location: impl Into<Location>) -> Result<Value> {
        self.check()?;
        self.value.get(location)
    }
    /// Applies an identity-addressed Change atomically against the current content.
    pub fn apply(&mut self, change: &Change) -> Result<()> {
        self.check()?;
        let value = apply(&self.value, change)?;
        let change = compose(&self.before, &self.change, change)?;
        self.value = value;
        self.change = change;
        Ok(())
    }
    /// Replaces content while preserving an existing target ID, or inserts a missing Map leaf.
    pub fn set(&mut self, location: impl Into<Location>, value: Value) -> Result<()> {
        self.check()?;
        let location = location.into();
        let existing = self.value.get(location.clone());
        let operation = if let Ok(existing) = &existing {
            Operation::Set {
                target: existing.id(),
                value: value.copy_into(Some(existing.id()))?,
            }
        } else if let Location::Path(mut path) = location {
            let Some(Segment::Key(key)) = path.pop() else {
                return Err(existing.unwrap_err());
            };
            let parent = self.value.get(path)?;
            if !matches!(parent.body(), Body::Map(_)) {
                return Err(super::change::type_error(
                    "set requires an existing Map parent",
                ));
            }
            Operation::Insert {
                destination: Destination {
                    parent: parent.id(),
                    slot: Segment::Key(key),
                },
                value: value.copied()?,
            }
        } else {
            return Err(existing.unwrap_err());
        };
        self.apply(&Change::new([operation])?)
    }
    /// Deletes an existing non-root element and its owned subtree.
    pub fn delete(&mut self, location: impl Into<Location>) -> Result<()> {
        let target = self.get(location)?.id();
        self.apply(&Change::new([Operation::Delete { target }])?)
    }
    /// Moves the same subtree; List destination indexes are interpreted after source removal.
    pub fn move_to(
        &mut self,
        source: impl Into<Location>,
        parent: impl Into<Location>,
        slot: Segment,
    ) -> Result<()> {
        let target = self.get(source)?.id();
        let parent = self.get(parent)?.id();
        self.apply(&Change::new([Operation::Move {
            target,
            destination: Destination { parent, slot },
        }])?)
    }
    /// Copies a subtree with fresh IDs and remapped internal Refs, returning the new root ID.
    pub fn copy(
        &mut self,
        source: impl Into<Location>,
        parent: impl Into<Location>,
        slot: Segment,
    ) -> Result<ElementId> {
        let value = self.get(source)?.copied()?;
        let parent = self.get(parent)?.id();
        let id = value.id();
        self.apply(&Change::new([Operation::Insert {
            destination: Destination { parent, slot },
            value,
        }])?)?;
        Ok(id)
    }
    /// Adds a checked i64 delta to an Int, retaining its ID.
    pub fn increment(&mut self, target: impl Into<Location>, delta: i64) -> Result<()> {
        let value = self.get(target)?;
        if !matches!(value.body(), Body::Int(_)) {
            return Err(super::change::type_error("expected Int"));
        }
        let target = value.id();
        self.apply(&Change::new([Operation::Add { target, delta }])?)
    }
    /// Replaces a List range with copies; zero count inserts and an empty input deletes.
    pub fn list_replace(
        &mut self,
        target: impl Into<Location>,
        index: usize,
        count: usize,
        values: Vec<Value>,
    ) -> Result<()> {
        let value = self.get(target)?;
        let Body::List(list) = value.body() else {
            return Err(super::change::type_error("expected List"));
        };
        let end = checked_end(index, count, list.len())?;
        let mut ops = list[index..end]
            .iter()
            .map(|v| Operation::Delete { target: v.id() })
            .collect::<Vec<_>>();
        for (offset, inserted) in values.into_iter().enumerate() {
            ops.push(Operation::Insert {
                destination: Destination {
                    parent: value.id(),
                    slot: Segment::Index(index.checked_add(offset).ok_or_else(|| {
                        Error::new(ErrorCode::LimitExceeded, "List index overflow")
                    })?),
                },
                value: inserted.copied()?,
            });
        }
        self.apply(&Change::new(ops)?)
    }
    /// Replaces a Text range using Unicode scalar coordinates in current working content.
    pub fn text_replace(
        &mut self,
        target: impl Into<Location>,
        index: usize,
        count: usize,
        text: &str,
    ) -> Result<()> {
        let value = self.get(target)?;
        let Body::Text(before) = value.body() else {
            return Err(super::change::type_error("expected Text"));
        };
        checked_end(index, count, before.chars().count())?;
        self.apply(&Change::new([Operation::Text {
            target: value.id(),
            change: TextChange::from_ops([
                TextOp::Retain(index),
                TextOp::Delete(count),
                TextOp::Insert(text.into()),
            ])?,
        }])?)
    }
    /// Applies scalar RichText operations to the current working content.
    pub fn rich_text_edit(
        &mut self,
        target: impl Into<Location>,
        operations: Vec<RichOp>,
    ) -> Result<()> {
        let value = self.get(target)?;
        if !matches!(value.body(), Body::RichText(_)) {
            return Err(super::change::type_error("expected RichText"));
        }
        let target = value.id();
        self.apply(&Change::new([Operation::RichText { target, operations }])?)
    }
    /// Converts a working Text/RichText offset, rejecting surrogate splits and out-of-bounds positions.
    pub fn utf16_to_scalar(&self, target: impl Into<Location>, position: usize) -> Result<usize> {
        let value = self.get(target)?;
        let result = match value.body() {
            Body::Text(text) => {
                crate::sequence::value::Text::new(text.clone()).utf16_to_code_point(position)
            }
            Body::RichText(spans) => super::rich::value_to_old(spans)?
                .as_rich_text()
                .unwrap()
                .utf16_to_code_point(position),
            _ => {
                return Err(super::change::type_error(
                    "UTF-16 coordinates require Text or RichText",
                ))
            }
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
            edit_steps: change.operations().to_vec(),
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
