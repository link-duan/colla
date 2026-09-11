use super::{
    apply, codec, compose,
    editing::{invalid_state, Phase, State},
    history::HistoryData,
    transform, Change, Document, EditResult, Error, ErrorCode, Origin, Priority, Result, Value,
};
use cocodec::{Decode, Encode};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq)]
/// Document identity, server revision and confirmed content.
pub struct SyncSnapshot {
    document_id: String,
    revision: u64,
    value: Value,
}
codec::record_codec!(SyncSnapshot, document_id, revision, value);
impl SyncSnapshot {
    /// Borrows the application document identity.
    pub fn document_id(&self) -> &str {
        &self.document_id
    }
    /// Returns the confirmed server revision, distinct from local version.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Borrows the confirmed immutable Value.
    pub fn value(&self) -> &Value {
        &self.value
    }
    /// Returns independent canonical bytes in a typed version-2 envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(3, self)
    }
    /// Strictly decodes and validates a typed version-2 envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Self = codec::decode(3, bytes)?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        identity(&self.document_id)?;
        self.value.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Controlled immutable request identity, original base revision and Change.
pub struct Submission {
    document_id: String,
    client_id: String,
    sequence: u64,
    base_revision: u64,
    change: Change,
}
codec::record_codec!(
    Submission,
    document_id,
    client_id,
    sequence,
    base_revision,
    change
);
impl Submission {
    /// Borrows the application document identity.
    pub fn document_id(&self) -> &str {
        &self.document_id
    }
    /// Borrows the application-provided writer identity.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    /// Returns the positive monotonic sequence number.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Returns the original server revision on which this request was created.
    pub fn base_revision(&self) -> u64 {
        self.base_revision
    }
    /// Borrows the immutable Change carried by this protocol object.
    pub fn change(&self) -> &Change {
        &self.change
    }
    /// Returns independent canonical bytes in a typed version-2 envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(4, self)
    }
    /// Strictly decodes and validates a typed version-2 envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Self = codec::decode(4, bytes)?;
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        identity(&self.document_id)?;
        identity(&self.client_id)?;
        if self.sequence == 0 {
            return Err(Error::new(
                ErrorCode::InvalidEncoding,
                "request sequence must be positive",
            ));
        }
        validate_change(&self.change)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Authority-ordered formal submission result; also confirms the submitting client.
pub struct Commit {
    document_id: String,
    client_id: String,
    sequence: u64,
    revision: u64,
    change: Change,
}
codec::record_codec!(Commit, document_id, client_id, sequence, revision, change);
impl Commit {
    /// Borrows the application document identity.
    pub fn document_id(&self) -> &str {
        &self.document_id
    }
    /// Borrows the application-provided writer identity.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    /// Returns the positive monotonic sequence number.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Returns the confirmed server revision, distinct from local version.
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Borrows the immutable Change carried by this protocol object.
    pub fn change(&self) -> &Change {
        &self.change
    }
    fn validate(&self) -> Result<()> {
        identity(&self.document_id)?;
        identity(&self.client_id)?;
        if self.sequence == 0 || self.revision == 0 {
            return Err(Error::new(
                ErrorCode::InvalidEncoding,
                "Commit sequence and revision must be positive",
            ));
        }
        validate_change(&self.change)
    }
}
#[derive(Debug, Clone, PartialEq)]
/// Controlled refusal of a submission with a structured recovery reason.
pub struct Rejection {
    document_id: String,
    client_id: String,
    sequence: u64,
    reason: Error,
}
codec::record_codec!(Rejection, document_id, client_id, sequence, reason);
impl Rejection {
    /// Borrows the application document identity.
    pub fn document_id(&self) -> &str {
        &self.document_id
    }
    /// Borrows the application-provided writer identity.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    /// Returns the positive monotonic sequence number.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    /// Borrows the structured rejection reason.
    pub fn reason(&self) -> &Error {
        &self.reason
    }
}
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
/// A controlled formal Commit or submission rejection.
pub enum ServerMessage {
    #[cocodec(tag = 0)]
    /// A formal ordered commit, including confirmation of the sender.
    Commit(Commit),
    #[cocodec(tag = 1)]
    /// A refused request whose local work must be retained.
    Rejection(Rejection),
}
impl ServerMessage {
    /// Returns independent canonical bytes in a typed version-2 envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(5, self)
    }
    /// Strictly decodes and validates a typed version-2 envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Self = codec::decode(5, bytes)?;
        match &value {
            Self::Commit(commit) => commit.validate()?,
            Self::Rejection(rejection) => {
                identity(&rejection.document_id)?;
                identity(&rejection.client_id)?;
                if rejection.sequence == 0 {
                    return Err(Error::new(
                        ErrorCode::InvalidEncoding,
                        "invalid rejection sequence",
                    ));
                }
            }
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Receipt {
    submission: Submission,
    commit: Commit,
}
codec::record_codec!(Receipt, submission, commit);
#[derive(Debug, Clone, PartialEq, Eq)]
/// Authority history floor, retained commits and request deduplication receipts.
pub struct AuthorityCheckpoint {
    floor: SyncSnapshot,
    log: Vec<Commit>,
    receipts: BTreeMap<(String, u64), Receipt>,
}
codec::record_codec!(AuthorityCheckpoint, floor, log, receipts);
impl AuthorityCheckpoint {
    /// Returns independent canonical bytes in a typed version-2 envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(8, self)
    }
    /// Strictly decodes and validates a typed version-2 envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Self = codec::decode(8, bytes)?;
        Authority::restore(value.clone())?;
        Ok(value)
    }
}
#[derive(Debug, Clone)]
/// Immutable centralized ordering and rebasing state; accept returns a new state.
pub struct Authority {
    data: Arc<AuthorityCheckpoint>,
    value: Value,
}
impl Authority {
    /// Creates synchronization state from a controlled content basis and application identity.
    pub fn create(document_id: impl Into<String>, value: Value) -> Result<Self> {
        let document_id = document_id.into();
        identity(&document_id)?;
        value.validate()?;
        Ok(Self {
            data: Arc::new(AuthorityCheckpoint {
                floor: SyncSnapshot {
                    document_id,
                    revision: 0,
                    value: value.clone(),
                },
                log: Vec::new(),
                receipts: BTreeMap::new(),
            }),
            value,
        })
    }
    /// Returns the confirmed server revision, distinct from local version.
    pub fn revision(&self) -> u64 {
        self.data
            .log
            .last()
            .map_or(self.data.floor.revision, |commit| commit.revision)
    }
    /// Returns the current confirmed SyncSnapshot, including server revision and document ID.
    pub fn snapshot(&self) -> SyncSnapshot {
        SyncSnapshot {
            document_id: self.data.floor.document_id.clone(),
            revision: self.revision(),
            value: self.value.clone(),
        }
    }
    /// Returns a complete immutable checkpoint suitable for strict encode and restore.
    pub fn checkpoint(&self) -> AuthorityCheckpoint {
        (*self.data).clone()
    }
    /// Validates and restores complete runtime state from a controlled checkpoint.
    pub fn restore(checkpoint: AuthorityCheckpoint) -> Result<Self> {
        checkpoint.floor.validate()?;
        let mut value = checkpoint.floor.value.clone();
        let mut revision = checkpoint.floor.revision;
        for commit in &checkpoint.log {
            commit.validate()?;
            if commit.document_id != checkpoint.floor.document_id
                || revision.checked_add(1) != Some(commit.revision)
            {
                return Err(Error::new(
                    ErrorCode::InvalidEncoding,
                    "noncontiguous Authority log",
                ));
            }
            value = apply(&value, &commit.change)?;
            revision = commit.revision;
            if checkpoint
                .receipts
                .get(&(commit.client_id.clone(), commit.sequence))
                .map(|r| &r.commit)
                != Some(commit)
            {
                return Err(Error::new(
                    ErrorCode::InvalidEncoding,
                    "missing Commit receipt",
                ));
            }
        }
        for ((client, seq), receipt) in &checkpoint.receipts {
            receipt.submission.validate()?;
            receipt.commit.validate()?;
            if client != &receipt.submission.client_id
                || seq != &receipt.submission.sequence
                || client != &receipt.commit.client_id
                || seq != &receipt.commit.sequence
                || receipt.commit.document_id != checkpoint.floor.document_id
                || receipt.submission.document_id != checkpoint.floor.document_id
                || receipt.commit.revision > revision
                || receipt.submission.base_revision >= receipt.commit.revision
            {
                return Err(Error::new(
                    ErrorCode::InvalidEncoding,
                    "invalid Authority receipt",
                ));
            }
        }
        Ok(Self {
            data: Arc::new(checkpoint),
            value,
        })
    }
    /// Returns retained commits after a revision, or reports unavailable history.
    pub fn commits_since(&self, revision: u64) -> Result<Vec<Commit>> {
        self.value_at(revision)?;
        Ok(self
            .data
            .log
            .iter()
            .filter(|commit| commit.revision > revision)
            .cloned()
            .collect())
    }
    /// Returns a new Authority with an advanced history floor and preserved deduplication receipts.
    pub fn compact(&self, through_revision: u64) -> Result<Self> {
        let value = self.value_at(through_revision)?;
        let mut data = (*self.data).clone();
        data.floor = SyncSnapshot {
            document_id: data.floor.document_id.clone(),
            revision: through_revision,
            value,
        };
        data.log.retain(|commit| commit.revision > through_revision);
        Ok(Self {
            data: Arc::new(data),
            value: self.value.clone(),
        })
    }
    fn value_at(&self, revision: u64) -> Result<Value> {
        if revision < self.data.floor.revision {
            return Err(Error::new(
                ErrorCode::HistoryExpired,
                "requested revision precedes retained history",
            )
            .detail("floorRevision", self.data.floor.revision.to_string()));
        }
        if revision > self.revision() {
            return Err(Error::new(
                ErrorCode::MissingRevision,
                "requested revision is not committed",
            ));
        }
        let mut value = self.data.floor.value.clone();
        for commit in self.data.log.iter().take_while(|c| c.revision <= revision) {
            value = apply(&value, &commit.change)?;
        }
        Ok(value)
    }
    /// Returns a new Authority and Commit/Rejection without mutating this Authority; persist before adopting.
    pub fn accept(&self, submission: &Submission) -> Result<(Self, ServerMessage)> {
        submission.validate()?;
        let reject = |reason| {
            (
                self.clone(),
                ServerMessage::Rejection(Rejection {
                    document_id: submission.document_id.clone(),
                    client_id: submission.client_id.clone(),
                    sequence: submission.sequence,
                    reason,
                }),
            )
        };
        if submission.document_id != self.data.floor.document_id {
            return Ok(reject(Error::new(
                ErrorCode::InvalidArgument,
                "submission document differs",
            )));
        }
        let key = (submission.client_id.clone(), submission.sequence);
        if let Some(receipt) = self.data.receipts.get(&key) {
            if receipt.submission != *submission {
                return Ok(reject(Error::new(
                    ErrorCode::InvalidArgument,
                    "request identity reused with different payload",
                )));
            }
            return Ok((self.clone(), ServerMessage::Commit(receipt.commit.clone())));
        }
        let last = self
            .data
            .receipts
            .keys()
            .filter(|(client, _)| client == &submission.client_id)
            .map(|(_, sequence)| *sequence)
            .max()
            .unwrap_or(0);
        if last.checked_add(1) != Some(submission.sequence) {
            return Ok(reject(Error::new(
                ErrorCode::InvalidArgument,
                "submission sequence is not the next client sequence",
            )));
        }
        let rebase = || -> Result<(Value, Change)> {
            let mut base = self.value_at(submission.base_revision)?;
            let mut change = submission.change.clone();
            apply(&base, &change)?;
            for commit in self
                .data
                .log
                .iter()
                .filter(|c| c.revision > submission.base_revision)
            {
                (change, _) = transform(&base, &change, &commit.change, Priority::Right)?;
                base = apply(&base, &commit.change)?;
            }
            let (change, value) = change.normalized(&self.value)?;
            Ok((value, change))
        };
        let (value, change) = match rebase() {
            Ok(result) => result,
            Err(error) => return Ok(reject(error)),
        };
        let revision = self
            .revision()
            .checked_add(1)
            .ok_or_else(|| Error::new(ErrorCode::LimitExceeded, "Authority revision exhausted"))?;
        let commit = Commit {
            document_id: submission.document_id.clone(),
            client_id: submission.client_id.clone(),
            sequence: submission.sequence,
            revision,
            change,
        };
        let mut data = (*self.data).clone();
        data.log.push(commit.clone());
        data.receipts.insert(
            key,
            Receipt {
                submission: submission.clone(),
                commit: commit.clone(),
            },
        );
        Ok((
            Self {
                data: Arc::new(data),
                value,
            },
            ServerMessage::Commit(commit),
        ))
    }
}

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
        codec::encode(6, self)
    }
    /// Strictly decodes and validates a typed version-2 envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Self = codec::decode(6, bytes)?;
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
            .sync
            .as_ref()
            .unwrap()
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
            .sync
            .as_ref()
            .unwrap()
            .recovery
            .clone())
    }
    /// Returns the original in-flight request without mutation, or None when idle/recovering.
    pub fn outbound(&self) -> Result<Option<Submission>> {
        self.document.readable()?;
        let state = self.document.shared.state.borrow();
        let sync = state.sync.as_ref().unwrap();
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
            session: state.sync.clone().unwrap(),
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
        let mut sync = state.sync.clone().unwrap();
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
                        .sync
                        .as_mut()
                        .unwrap()
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
                    .sync
                    .as_mut()
                    .unwrap()
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
fn identity(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 4096 {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "document/client identity must contain 1..4096 UTF-8 bytes",
        ));
    }
    Ok(())
}
fn validate_change(change: &Change) -> Result<()> {
    if &Change::new(change.operations().iter().cloned())? != change {
        return Err(Error::new(
            ErrorCode::InvalidEncoding,
            "noncanonical change",
        ));
    }
    Ok(())
}
