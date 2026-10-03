use super::super::{apply, codec, transform, Change, Error, ErrorCode, Priority, Result, Value};
use super::{identity, Commit, Rejection, ServerMessage, Submission, SyncSnapshot};
use cocodec::{Decode, Encode};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
struct Receipt {
    submission: Submission,
    commit: Commit,
}
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
/// Authority history floor, retained commits and request deduplication receipts.
pub struct AuthorityCheckpoint {
    floor: SyncSnapshot,
    log: Vec<Commit>,
    receipts: BTreeMap<(String, u64), Receipt>,
}
impl AuthorityCheckpoint {
    /// Returns independent canonical bytes in a typed binary envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(codec::Kind::AuthorityCheckpoint, self)
    }
    /// Strictly decodes a typed binary envelope, rejecting trailing data;
    /// `restore` validates the checkpoint's semantic consistency.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        codec::decode(codec::Kind::AuthorityCheckpoint, bytes)
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
