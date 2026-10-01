use super::super::{
    apply, codec, compose,
    editing::{invalid_state, Phase, State},
    history::HistoryData,
    transform, Change, Document, EditResult, Error, ErrorCode, Origin, Priority, Result, Value,
};
use super::{identity, validate_change, Commit, ServerMessage, Submission, SyncSnapshot};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Pending {
    original: Submission,
    original_base: SyncSnapshot,
    rebased: Change,
}
codec::record_codec!(Pending, original, original_base, rebased);
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SessionData {
    client_id: String,
    next_sequence: u64,
    confirmed: SyncSnapshot,
    pending: Option<Pending>,
    buffer: Change,
    recovery: Option<Error>,
    received: BTreeMap<u64, Commit>,
}
codec::record_codec!(
    SessionData,
    client_id,
    next_sequence,
    confirmed,
    pending,
    buffer,
    recovery,
    received
);
impl SessionData {
    pub(crate) fn enqueue(&mut self, change: &Change) -> Result<()> {
        if self.recovery.is_some() || change.is_noop() {
            return Ok(());
        }
        if let Some(pending) = &self.pending {
            let base = apply(&self.confirmed.value, &pending.rebased)?;
            self.buffer = compose(&base, &self.buffer, change)?;
        } else {
            self.promote(change.clone())?;
        }
        Ok(())
    }
    fn promote(&mut self, change: Change) -> Result<()> {
        let sequence = self.next_sequence;
        self.next_sequence = sequence
            .checked_add(1)
            .ok_or_else(|| Error::new(ErrorCode::LimitExceeded, "submission sequence exhausted"))?;
        self.pending = Some(Pending {
            original: Submission {
                document_id: self.confirmed.document_id.clone(),
                client_id: self.client_id.clone(),
                sequence,
                base_revision: self.confirmed.revision,
                change: change.clone(),
            },
            original_base: self.confirmed.clone(),
            rebased: change,
        });
        Ok(())
    }
    fn validate(&self, visible: &Value) -> Result<()> {
        identity(&self.client_id)?;
        self.confirmed.validate()?;
        validate_change(&self.buffer)?;
        if self.next_sequence == 0 {
            return Err(Error::new(
                ErrorCode::InvalidEncoding,
                "invalid next sequence",
            ));
        }
        let mut value = self.confirmed.value.clone();
        if let Some(pending) = &self.pending {
            pending.original.validate()?;
            validate_change(&pending.rebased)?;
            if pending.original.client_id != self.client_id
                || pending.original.document_id != self.confirmed.document_id
                || pending.original.sequence.checked_add(1) != Some(self.next_sequence)
                || pending.original.base_revision > self.confirmed.revision
            {
                return Err(Error::new(
                    ErrorCode::InvalidEncoding,
                    "invalid in-flight identity or revision",
                ));
            }
            value = apply(&value, &pending.rebased)?;
            pending.original_base.validate()?;
            if pending.original_base.document_id != self.confirmed.document_id
                || pending.original_base.revision != pending.original.base_revision
            {
                return Err(Error::new(
                    ErrorCode::InvalidEncoding,
                    "original request basis differs",
                ));
            }
            let mut basis = pending.original_base.value.clone();
            let mut expected = pending.original.change.clone();
            apply(&basis, &expected)?;
            for revision in pending
                .original
                .base_revision
                .checked_add(1)
                .into_iter()
                .flat_map(|start| start..=self.confirmed.revision)
            {
                let committed = self.received.get(&revision).ok_or_else(|| {
                    Error::new(
                        ErrorCode::InvalidEncoding,
                        "missing in-flight rebase record",
                    )
                })?;
                (expected, _) = transform(&basis, &expected, &committed.change, Priority::Right)?;
                basis = apply(&basis, &committed.change)?;
            }
            if basis != self.confirmed.value || apply(&basis, &expected)? != value {
                return Err(Error::new(
                    ErrorCode::InvalidEncoding,
                    "in-flight payload and rebase state disagree",
                ));
            }
        } else if !self.buffer.is_noop() {
            return Err(Error::new(
                ErrorCode::InvalidEncoding,
                "buffer without in-flight request",
            ));
        }
        value = apply(&value, &self.buffer)?;
        if self.recovery.is_none() && value != *visible {
            return Err(Error::new(
                ErrorCode::InvalidEncoding,
                "checkpoint visible content differs from pending projection",
            ));
        }
        for (revision, commit) in &self.received {
            commit.validate()?;
            if revision != &commit.revision
                || *revision > self.confirmed.revision
                || commit.document_id != self.confirmed.document_id
            {
                return Err(Error::new(
                    ErrorCode::InvalidEncoding,
                    "invalid received Commit record",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Complete recoverable client state, including pending, buffer, content and enabled History.
pub struct SessionCheckpoint {
    value: Value,
    version: u64,
    history: Option<HistoryData>,
    session: SessionData,
}
codec::record_codec!(SessionCheckpoint, value, version, history, session);
impl SessionCheckpoint {
    /// Returns independent canonical bytes in a typed version-2 envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(codec::Kind::SessionCheckpoint, self)
    }
    /// Strictly decodes and validates a typed version-2 envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Self = codec::decode(codec::Kind::SessionCheckpoint, bytes)?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        self.value.validate()?;
        self.session.validate(&self.value)?;
        if let Some(history) = &self.history {
            history.validate(&self.value)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
/// Client synchronization runtime with one in-flight request and one composed buffer.
pub struct SyncSession {
    document: Document,
}
impl SyncSession {
    /// Creates synchronization state from a controlled content basis and application identity.
    pub fn create(client_id: impl Into<String>, snapshot: SyncSnapshot) -> Result<Self> {
        let client_id = client_id.into();
        identity(&client_id)?;
        snapshot.validate()?;
        let document = Document::create(snapshot.value.clone())?;
        document.shared.state.borrow_mut().sync = Some(SessionData {
            client_id,
            next_sequence: 1,
            confirmed: snapshot,
            pending: None,
            buffer: Change::noop(),
            recovery: None,
            received: BTreeMap::new(),
        });
        Ok(Self { document })
    }
    /// Returns this session's shared visible Document.
    pub fn document(&self) -> Document {
        self.document.clone()
    }
    /// Returns the confirmed server revision, distinct from local version.
    pub fn revision(&self) -> Result<u64> {
        self.document.readable()?;
        Ok(self
            .document
            .shared
            .state
            .borrow()
            .session()?
            .confirmed
            .revision)
    }
    /// Returns the recovery diagnostic when safe synchronization cannot continue.
    pub fn recovery_reason(&self) -> Result<Option<Error>> {
        self.document.readable()?;
        Ok(self
            .document
            .shared
            .state
            .borrow()
            .session()?
            .recovery
            .clone())
    }
    /// Returns the original in-flight request without mutation, or None when idle/recovering.
    pub fn outbound(&self) -> Result<Option<Submission>> {
        self.document.readable()?;
        let state = self.document.shared.state.borrow();
        let sync = state.session()?;
        Ok(if sync.recovery.is_some() {
            None
        } else {
            sync.pending.as_ref().map(|p| p.original.clone())
        })
    }
    /// Returns a complete immutable checkpoint suitable for strict encode and restore.
    pub fn checkpoint(&self) -> Result<SessionCheckpoint> {
        self.document.idle()?;
        let state = self.document.shared.state.borrow();
        Ok(SessionCheckpoint {
            value: state.value.clone(),
            version: state.version,
            history: state.history.clone(),
            session: state.session()?.clone(),
        })
    }
    /// Validates and restores complete runtime state from a controlled checkpoint.
    pub fn restore(checkpoint: SessionCheckpoint) -> Result<Self> {
        checkpoint.validate()?;
        let document = Document::create(checkpoint.value.clone())?;
        *document.shared.state.borrow_mut() = State {
            value: checkpoint.value,
            version: checkpoint.version,
            history_epoch: u64::from(checkpoint.history.is_some()),
            history: checkpoint.history,
            sync: Some(checkpoint.session),
        };
        Ok(Self { document })
    }
    /// Closes this runtime idempotently; previously returned immutable content remains valid.
    pub fn close(&self) -> Result<()> {
        self.document.close()
    }
    /// Atomically receives an ordered Commit or rejection; gaps leave content unchanged.
    pub fn receive(&self, message: &ServerMessage) -> Result<Option<EditResult>> {
        self.document.idle()?;
        let state = self.document.shared.state.borrow().clone();
        let mut sync = state.session()?.clone();
        if sync.recovery.is_some() {
            return Err(invalid_state("session requires recovery"));
        }
        let commit = match message {
            ServerMessage::Rejection(rejection) => {
                if rejection.document_id == sync.confirmed.document_id
                    && sync.pending.as_ref().is_some_and(|p| {
                        p.original.client_id == rejection.client_id
                            && p.original.sequence == rejection.sequence
                    })
                {
                    self.document
                        .shared
                        .state
                        .borrow_mut()
                        .session_mut()?
                        .recovery = Some(rejection.reason.clone());
                }
                return Ok(None);
            }
            ServerMessage::Commit(commit) => commit,
        };
        commit.validate()?;
        if commit.document_id != sync.confirmed.document_id {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "Commit document differs",
            ));
        }
        if commit.revision <= sync.confirmed.revision {
            if sync
                .received
                .get(&commit.revision)
                .is_some_and(|old| old != commit)
            {
                return Err(Error::new(
                    ErrorCode::InvalidArgument,
                    "revision reused with different Commit",
                ));
            }
            return Ok(None);
        }
        if sync.confirmed.revision.checked_add(1) != Some(commit.revision) {
            return Err(Error::new(ErrorCode::MissingRevision, "Commit gap")
                .detail("fromRevision", (sync.confirmed.revision + 1).to_string())
                .detail("throughRevision", (commit.revision - 1).to_string()));
        }
        let mut calculate = || -> Result<(State, Option<EditResult>)> {
            let confirmed_value = apply(&sync.confirmed.value, &commit.change)?;
            let mut visible_remote = commit.change.clone();
            let own = commit.client_id == sync.client_id;
            if own {
                let pending = sync
                    .pending
                    .as_ref()
                    .ok_or_else(|| invalid_state("own Commit has no in-flight request"))?;
                if pending.original.sequence != commit.sequence
                    || apply(&sync.confirmed.value, &pending.rebased)? != confirmed_value
                {
                    return Err(invalid_state(
                        "own Commit identity or accepted payload differs",
                    ));
                }
                sync.pending = None;
                visible_remote = Change::noop();
            } else if let Some(pending) = &mut sync.pending {
                let optimistic_base = apply(&sync.confirmed.value, &pending.rebased)?;
                (pending.rebased, visible_remote) = transform(
                    &sync.confirmed.value,
                    &pending.rebased,
                    &visible_remote,
                    Priority::Right,
                )?;
                (sync.buffer, visible_remote) = transform(
                    &optimistic_base,
                    &sync.buffer,
                    &visible_remote,
                    Priority::Right,
                )?;
            }
            sync.confirmed = SyncSnapshot {
                document_id: sync.confirmed.document_id.clone(),
                revision: commit.revision,
                value: confirmed_value,
            };
            if own && !sync.buffer.is_noop() {
                let buffered = std::mem::take(&mut sync.buffer);
                sync.promote(buffered)?;
            }
            let (visible_remote, _) = visible_remote.normalized(&state.value)?;
            let (mut next, edit) = if visible_remote.is_noop() {
                let mut next = state.clone();
                if let Some(history) = &mut next.history {
                    history.group = None;
                }
                (next, None)
            } else {
                let (next, edit) = state.prepare(&visible_remote, Origin::Remote, None)?;
                (next, Some(edit))
            };
            sync.received.insert(commit.revision, commit.clone());
            // Only the original in-flight basis needs its intervening commits.
            // A caught-up session retains one receipt, not the document's log.
            let retain_from = sync.pending.as_ref().map_or(commit.revision, |pending| {
                pending.original.base_revision.saturating_add(1)
            });
            sync.received.retain(|revision, _| *revision >= retain_from);
            sync.validate(&next.value)?;
            next.sync = Some(sync.clone());
            Ok((next, edit))
        };
        match calculate() {
            Ok((next, result)) => {
                *self.document.shared.state.borrow_mut() = next;
                Ok(result)
            }
            Err(error) => {
                self.document
                    .shared
                    .state
                    .borrow_mut()
                    .session_mut()?
                    .recovery = Some(error.clone());
                Err(error)
            }
        }
    }
    /// Returns whether the session runtime has closed.
    pub fn is_closed(&self) -> bool {
        self.document.shared.phase.get() == Phase::Closed
    }
}
