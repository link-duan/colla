use super::super::{codec, Change, Error, ErrorCode, Result, Value};
use super::{identity, validate_change};
use cocodec::{Decode, Encode};

#[derive(Debug, Clone, PartialEq, Eq)]
/// Document identity, server revision and confirmed content.
pub struct SyncSnapshot {
    pub(super) document_id: String,
    pub(super) revision: u64,
    pub(super) value: Value,
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
    /// Returns independent canonical bytes in a typed binary envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(codec::Kind::SyncSnapshot, self)
    }
    /// Strictly decodes and validates a typed binary envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Self = codec::decode(codec::Kind::SyncSnapshot, bytes)?;
        value.validate()?;
        Ok(value)
    }
    pub(super) fn validate(&self) -> Result<()> {
        identity(&self.document_id)?;
        self.value.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Controlled immutable request identity, original base revision and Change.
pub struct Submission {
    pub(super) document_id: String,
    pub(super) client_id: String,
    pub(super) sequence: u64,
    pub(super) base_revision: u64,
    pub(super) change: Change,
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
    /// Returns independent canonical bytes in a typed binary envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(codec::Kind::Submission, self)
    }
    /// Strictly decodes and validates a typed binary envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Self = codec::decode(codec::Kind::Submission, bytes)?;
        value.validate()?;
        Ok(value)
    }
    pub(super) fn validate(&self) -> Result<()> {
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
    pub(super) document_id: String,
    pub(super) client_id: String,
    pub(super) sequence: u64,
    pub(super) revision: u64,
    pub(super) change: Change,
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
    pub(super) fn validate(&self) -> Result<()> {
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
    pub(super) document_id: String,
    pub(super) client_id: String,
    pub(super) sequence: u64,
    pub(super) reason: Error,
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
    /// Returns independent canonical bytes in a typed binary envelope.
    pub fn encode(&self) -> Vec<u8> {
        codec::encode(codec::Kind::ServerMessage, self)
    }
    /// Strictly decodes and validates a typed binary envelope, rejecting trailing data.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Self = codec::decode(codec::Kind::ServerMessage, bytes)?;
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
