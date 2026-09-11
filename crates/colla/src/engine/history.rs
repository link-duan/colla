use super::{
    apply, codec, compose,
    editing::{invalid_state, Phase},
    transform, Change, Document, EditResult, Error, ErrorCode, Origin, Priority, Result, Value,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HistoryData {
    pub(crate) undo: Vec<Change>,
    pub(crate) redo: Vec<Change>,
    pub(crate) capacity: usize,
    pub(crate) group: Option<String>,
}
codec::record_codec!(HistoryData, undo, redo, capacity, group);
impl HistoryData {
    pub(crate) fn record(&mut self, edit: &EditResult, group: Option<String>) -> Result<()> {
        self.redo.clear();
        if group.is_some() && self.group == group && !self.undo.is_empty() {
            let previous = self.undo.pop().unwrap();
            self.undo
                .push(compose(&edit.after, &edit.inverse, &previous)?);
        } else {
            self.undo.push(edit.inverse.clone());
        }
        if self.undo.len() > self.capacity {
            self.undo.remove(0);
        }
        self.group = group;
        Ok(())
    }
    pub(crate) fn rebase(&mut self, base: &Value, change: &Change) -> Result<()> {
        fn stack(stack: &mut [Change], base: &Value, change: &Change) -> Result<()> {
            let mut base = base.clone();
            let mut remote = change.clone();
            for inverse in stack.iter_mut().rev() {
                let (rebased, remote_before_inverse) =
                    transform(&base, inverse, &remote, Priority::Right)?;
                base = apply(&base, inverse)?;
                *inverse = rebased;
                remote = remote_before_inverse;
            }
            Ok(())
        }
        stack(&mut self.undo, base, change)?;
        stack(&mut self.redo, base, change)?;
        self.group = None;
        Ok(())
    }
    pub(crate) fn validate(&self, base: &Value) -> Result<()> {
        if self.capacity == 0
            || self.capacity > 1_000_000
            || self.undo.len() > self.capacity
            || self.redo.len() > self.capacity
        {
            return Err(Error::new(
                ErrorCode::InvalidEncoding,
                "invalid History capacity",
            ));
        }
        for stack in [&self.undo, &self.redo] {
            let mut value = base.clone();
            for change in stack.iter().rev() {
                value = apply(&value, change)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
/// Document-attached collaborative undo and redo runtime.
pub struct History {
    document: Document,
    epoch: u64,
}
impl History {
    /// Attaches History with 100 undo units, or returns the existing History.
    pub fn attach(document: &Document) -> Result<Self> {
        Self::attach_with_capacity(document, 100)
    }
    /// Attaches History with an explicit positive capacity, preserving an existing attachment.
    pub fn attach_with_capacity(document: &Document, capacity: usize) -> Result<Self> {
        document.idle()?;
        let mut state = document.shared.state.borrow_mut();
        if state.history.is_none() {
            if capacity == 0 || capacity > 1_000_000 {
                return Err(Error::new(
                    ErrorCode::InvalidArgument,
                    "History capacity must be between 1 and 1000000",
                ));
            }
            state.history_epoch = state.history_epoch.checked_add(1).ok_or_else(|| {
                Error::new(ErrorCode::LimitExceeded, "History generation exhausted")
            })?;
            state.history = Some(HistoryData {
                undo: Vec::new(),
                redo: Vec::new(),
                capacity,
                group: None,
            });
        }
        Ok(Self {
            document: document.clone(),
            epoch: state.history_epoch,
        })
    }
    fn check(&self) -> Result<()> {
        self.document.readable()?;
        let state = self.document.shared.state.borrow();
        if state.history_epoch != self.epoch || state.history.is_none() {
            return Err(invalid_state("History is closed"));
        }
        Ok(())
    }
    /// Returns whether a non-Noop undo unit is available.
    pub fn can_undo(&self) -> Result<bool> {
        self.check()?;
        Ok(self
            .document
            .shared
            .state
            .borrow()
            .history
            .as_ref()
            .unwrap()
            .undo
            .iter()
            .any(|c| !c.is_noop()))
    }
    /// Returns whether a non-Noop redo unit is available.
    pub fn can_redo(&self) -> Result<bool> {
        self.check()?;
        Ok(self
            .document
            .shared
            .state
            .borrow()
            .history
            .as_ref()
            .unwrap()
            .redo
            .iter()
            .any(|c| !c.is_noop()))
    }
    /// Atomically applies the newest effective inverse as a new local edit.
    pub fn undo(&self) -> Result<Option<EditResult>> {
        self.step(false)
    }
    /// Atomically reapplies the newest effective redo unit as a new local edit.
    pub fn redo(&self) -> Result<Option<EditResult>> {
        self.step(true)
    }
    fn step(&self, redo: bool) -> Result<Option<EditResult>> {
        self.check()?;
        self.document.idle()?;
        let mut candidate = self.document.shared.state.borrow().clone();
        loop {
            let history = candidate.history.as_mut().unwrap();
            history.group = None;
            let stack = if redo {
                &mut history.redo
            } else {
                &mut history.undo
            };
            let Some(change) = stack.pop() else {
                *self.document.shared.state.borrow_mut() = candidate;
                return Ok(None);
            };
            let (change, _) = change.normalized(&candidate.value)?;
            if change.is_noop() {
                continue;
            }
            let (mut next, result) = candidate.prepare(
                &change,
                if redo { Origin::Redo } else { Origin::Undo },
                None,
            )?;
            let history = next.history.as_mut().unwrap();
            if redo {
                history.undo.push(result.inverse.clone());
            } else {
                history.redo.push(result.inverse.clone());
            }
            *self.document.shared.state.borrow_mut() = next;
            return Ok(Some(result));
        }
    }
    /// Clears undo, redo and active grouping without changing content.
    pub fn clear(&self) -> Result<()> {
        self.check()?;
        self.document.idle()?;
        let mut state = self.document.shared.state.borrow_mut();
        let history = state.history.as_mut().unwrap();
        history.undo.clear();
        history.redo.clear();
        history.group = None;
        Ok(())
    }
    /// Closes this runtime idempotently; previously returned immutable content remains valid.
    pub fn close(&self) -> Result<()> {
        if self.document.shared.phase.get() == Phase::Closed {
            return Ok(());
        }
        self.document.idle()?;
        let mut state = self.document.shared.state.borrow_mut();
        if state.history_epoch == self.epoch {
            state.history = None;
        }
        Ok(())
    }
    /// Returns a complete immutable checkpoint suitable for strict encode and restore.
    pub fn checkpoint(&self) -> Result<HistoryCheckpoint> {
        self.check()?;
        self.document.idle()?;
        let state = self.document.shared.state.borrow();
        Ok(HistoryCheckpoint {
            base: state.value.clone(),
            history: state.history.clone().unwrap(),
        })
    }
    /// Validates and restores complete runtime state from a controlled checkpoint.
    pub fn restore(document: &Document, checkpoint: HistoryCheckpoint) -> Result<Self> {
        document.idle()?;
        checkpoint.history.validate(&checkpoint.base)?;
        if document.snapshot()? != checkpoint.base {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "History checkpoint content basis differs",
            ));
        }
        let history = Self::attach_with_capacity(document, checkpoint.history.capacity)?;
        document.shared.state.borrow_mut().history = Some(checkpoint.history);
        Ok(history)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
/// History stacks and exact content basis for controlled restoration.
pub struct HistoryCheckpoint {
    base: Value,
    history: HistoryData,
}
codec::record_codec!(HistoryCheckpoint, base, history);
impl HistoryCheckpoint {
    /// Returns independent canonical bytes in a typed version-2 envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(7, self)
    }
    /// Strictly decodes and validates a typed version-2 envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let checkpoint: Self = codec::decode(7, bytes)?;
        checkpoint.base.validate()?;
        checkpoint.history.validate(&checkpoint.base)?;
        Ok(checkpoint)
    }
}
