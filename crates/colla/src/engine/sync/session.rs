use super::super::{
    apply, codec, compose,
    editing::{invalid_state, Phase, State},
    history::HistoryData,
    transform, Change, Document, EditResult, Error, ErrorCode, Origin, Priority, Result, Value,
};
use super::{identity, Commit, ServerMessage, Submission, SyncSnapshot};
use cocodec::{Decode, Encode};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
struct Pending {
    original: Submission,
    original_base: SyncSnapshot,
    rebased: Change,
}
#[derive(Debug, Clone, Default, PartialEq, Encode, Decode)]
pub(crate) struct SessionData {
    client_id: String,
    next_sequence: u64,
    confirmed: SyncSnapshot,
    pending: Option<Pending>,
    buffer: Change,
    recovery: Option<Error>,
    received: BTreeMap<u64, Commit>,
}
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
        if self.next_sequence == 0 {
            return Err(Error::new(
                ErrorCode::InvalidEncoding,
                "invalid next sequence",
            ));
        }
        let mut value = self.confirmed.value.clone();
        if let Some(pending) = &self.pending {
            pending.original.validate()?;
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

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
/// Complete recoverable client state, including pending, buffer, content and enabled History.
pub struct SessionCheckpoint {
    value: Value,
    version: u64,
    history: Option<HistoryData>,
    session: SessionData,
}
impl SessionCheckpoint {
    /// Returns independent canonical bytes in a typed binary envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(codec::Kind::SessionCheckpoint, self)
    }
    /// Strictly decodes a typed binary envelope, rejecting trailing data;
    /// `restore` validates the checkpoint's semantic consistency.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        codec::decode(codec::Kind::SessionCheckpoint, bytes)
    }
    fn validate(&self) -> Result<()> {
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
        let document = Document::create(snapshot.value.clone());
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
        let document = Document::create(checkpoint.value.clone());
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

#[cfg(test)]
mod tests {
    use super::super::{Authority, Rejection};
    use super::*;
    use crate::engine::{Segment, Transaction};

    fn setup() -> (Authority, SyncSession, SyncSession) {
        let value = Value::map([("n".to_string(), Value::int(0))]).unwrap();
        let authority = Authority::create("doc", value).unwrap();
        let a = SyncSession::create("a", authority.snapshot()).unwrap();
        let b = SyncSession::create("b", authority.snapshot()).unwrap();
        (authority, a, b)
    }
    fn bump(tx: &mut Transaction) -> Result<()> {
        tx.increment(&[Segment::Key("n".into())], 1)
    }
    fn commit(authority: &mut Authority, session: &SyncSession) -> ServerMessage {
        let request = session.outbound().unwrap().unwrap();
        let (next, message) = authority.accept(&request).unwrap();
        *authority = next;
        message
    }

    #[test]
    fn commit_gap_is_reported_without_entering_recovery() {
        let (mut authority, a, b) = setup();
        a.document().edit(bump).unwrap();
        let first = commit(&mut authority, &a);
        a.receive(&first).unwrap();
        a.document().edit(bump).unwrap();
        let second = commit(&mut authority, &a);
        let before = b.document().snapshot().unwrap();
        let error = b.receive(&second).unwrap_err();
        assert_eq!(error.code, ErrorCode::MissingRevision);
        assert_eq!(b.document().snapshot().unwrap(), before);
        assert!(b.recovery_reason().unwrap().is_none());
        b.receive(&first).unwrap();
        b.receive(&second).unwrap();
        assert_eq!(b.revision().unwrap(), 2);
        assert_eq!(b.receive(&first).unwrap(), None, "duplicates are ignored");
    }

    #[test]
    fn rejection_of_the_in_flight_request_requires_recovery() {
        let (authority, a, _) = setup();
        a.document().edit(bump).unwrap();
        let request = a.outbound().unwrap().unwrap();
        let rejection = |sequence| {
            ServerMessage::Rejection(Rejection {
                document_id: "doc".into(),
                client_id: "a".into(),
                sequence,
                reason: Error::new(ErrorCode::IncompatibleChange, "test"),
            })
        };
        a.receive(&rejection(request.sequence + 1)).unwrap();
        assert!(a.recovery_reason().unwrap().is_none(), "unrelated sequence");
        a.receive(&rejection(request.sequence)).unwrap();
        assert_eq!(
            a.recovery_reason().unwrap().unwrap().code,
            ErrorCode::IncompatibleChange
        );
        assert_eq!(a.outbound().unwrap(), None);
        let (_, message) = authority.accept(&request).unwrap();
        assert_eq!(
            a.receive(&message).unwrap_err().code,
            ErrorCode::InvalidState
        );
    }

    #[test]
    fn foreign_commit_with_mismatched_own_identity_enters_recovery() {
        let (mut authority, a, b) = setup();
        b.document().edit(bump).unwrap();
        let ServerMessage::Commit(mut forged) = commit(&mut authority, &b) else {
            panic!("expected Commit");
        };
        forged.client_id = "a".into();
        let error = a.receive(&ServerMessage::Commit(forged)).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidState);
        assert!(a.recovery_reason().unwrap().is_some());
    }

    #[test]
    fn detached_state_reports_invalid_state_instead_of_panicking() {
        let document = Document::create(Value::int(0));
        let state = document.shared.state.borrow();
        assert_eq!(state.session().unwrap_err().code, ErrorCode::InvalidState);
        assert_eq!(state.history().unwrap_err().code, ErrorCode::InvalidState);
    }
}
