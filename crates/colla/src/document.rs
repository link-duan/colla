//! High-level document envelopes for local persistence and updates.

use crate::change::Change;
use crate::error::CodecError;
use crate::value::Value;

const SNAPSHOT_MAGIC: &[u8; 6] = b"COLLAS";
const UPDATE_MAGIC: &[u8; 6] = b"COLLAU";
const PROTOCOL_VERSION: u16 = 1;
const HEADER_LEN: usize = 8;

/// A persistable document content snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Snapshot {
    revision: u64,
    content: Value,
}

impl Snapshot {
    /// Creates a snapshot from a content Value and revision.
    pub fn new(revision: u64, content: Value) -> Self {
        Self { revision, content }
    }

    /// Returns the document revision represented by this snapshot.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Borrows the content Value.
    pub fn content(&self) -> &Value {
        &self.content
    }

    /// Encodes the local Snapshot envelope.
    pub fn encode(&self) -> Vec<u8> {
        let payload = cocodec::encode_to_vec(&(self.revision, self.content.clone()))
            .expect("encoding a Snapshot payload into a Vec is infallible");
        encode_envelope(SNAPSHOT_MAGIC, &payload)
    }

    /// Decodes a local Snapshot envelope.
    pub fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        let payload = decode_envelope(bytes, SNAPSHOT_MAGIC, "snapshot")?;
        let (revision, content): (u64, Value) =
            cocodec::decode_from_slice(payload).map_err(CodecError::from)?;
        Ok(Self { revision, content })
    }
}

/// A versioned document update exchanged by local and remote consumers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Update {
    revision: u64,
    update_id: u64,
    change: Change,
}

impl Update {
    /// Creates an update based on `revision`.
    pub fn new(revision: u64, update_id: u64, change: Change) -> Self {
        Self {
            revision,
            update_id,
            change,
        }
    }

    /// Returns the base revision for this update.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the local correlation identifier.
    pub fn update_id(&self) -> u64 {
        self.update_id
    }

    /// Borrows the contained Core Change.
    pub fn change(&self) -> &Change {
        &self.change
    }

    /// Encodes the local Update envelope.
    pub fn encode(&self) -> Vec<u8> {
        let payload = cocodec::encode_to_vec(&(self.revision, self.update_id, self.change.clone()))
            .expect("encoding an Update payload into a Vec is infallible");
        encode_envelope(UPDATE_MAGIC, &payload)
    }

    /// Decodes a local Update envelope.
    pub fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        let payload = decode_envelope(bytes, UPDATE_MAGIC, "update")?;
        let (revision, update_id, change): (u64, u64, Change) =
            cocodec::decode_from_slice(payload).map_err(CodecError::from)?;
        Ok(Self {
            revision,
            update_id,
            change,
        })
    }
}

fn encode_envelope(magic: &[u8; 6], payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER_LEN + payload.len());
    bytes.extend_from_slice(magic);
    bytes.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

fn decode_envelope<'a>(
    bytes: &'a [u8],
    magic: &[u8; 6],
    context: &'static str,
) -> Result<&'a [u8], CodecError> {
    if bytes.len() < HEADER_LEN {
        return Err(CodecError::UnexpectedEof {
            offset: bytes.len(),
        });
    }
    if &bytes[..6] != magic {
        return Err(CodecError::InvalidMagic { context });
    }
    let version = u16::from_le_bytes([bytes[6], bytes[7]]);
    if version != PROTOCOL_VERSION {
        return Err(CodecError::UnsupportedVersion { context, version });
    }
    Ok(&bytes[HEADER_LEN..])
}
