use super::{Error, ErrorCode, Result};
use cocodec::{Decode, Encode};

const MAGIC: &[u8] = b"COLLA";

// Version 2 records have required positional fields. In particular, missing
// identity/revision fields must never acquire defaults during decoding.
macro_rules! record_codec {
    ($ty:ty, $($field:ident),+ $(,)?) => {
        impl cocodec::Encode for $ty {
            fn encode<W: cocodec::Write>(&self, w: &mut W) -> std::result::Result<(), cocodec::Error> {
                $(cocodec::Encode::encode(&self.$field, w)?;)+
                Ok(())
            }
        }
        impl cocodec::Decode for $ty {
            fn decode<R: cocodec::Read>(d: &mut cocodec::Decoder<R>) -> std::result::Result<Self, cocodec::Error> {
                d.nested(|d| Ok(Self { $($field: cocodec::Decode::decode(d)?),+ }))
            }
        }
    };
}
pub(crate) use record_codec;

/// Envelope type byte. Tags are part of the version-2 wire format documented in
/// `docs/binary-format.md`; never renumber or reuse one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Kind {
    Value = 1,
    Change = 2,
    SyncSnapshot = 3,
    Submission = 4,
    ServerMessage = 5,
    SessionCheckpoint = 6,
    HistoryCheckpoint = 7,
    AuthorityCheckpoint = 8,
}

// A typed version-2 binary envelope around the Rust-owned canonical payload.
// No JavaScript code generates or parses protocol bytes.
pub(crate) fn encode<T: Encode>(kind: Kind, value: &T) -> Vec<u8> {
    let mut bytes = Vec::from(MAGIC);
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.push(kind as u8);
    value
        .encode(&mut bytes)
        .expect("writing to a Vec cannot fail");
    bytes
}
pub(crate) fn decode<T: Decode>(kind: Kind, bytes: &[u8]) -> Result<T> {
    if bytes.len() < 8
        || &bytes[..5] != MAGIC
        || bytes[5..7] != 2u16.to_le_bytes()
        || bytes[7] != kind as u8
    {
        return Err(Error::new(
            ErrorCode::InvalidEncoding,
            "invalid envelope magic, version, or kind",
        ));
    }
    cocodec::decode_from_slice(&bytes[8..])
        .map_err(|e| Error::new(ErrorCode::InvalidEncoding, e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_rejects_wrong_kind_version_and_magic() {
        let bytes = encode(Kind::Submission, &7u64);
        assert_eq!(decode::<u64>(Kind::Submission, &bytes).unwrap(), 7);
        assert_eq!(&bytes[..8], b"COLLA\x02\x00\x04");
        let mut wrong_version = bytes.clone();
        wrong_version[5] = 1;
        let mut wrong_magic = bytes.clone();
        wrong_magic[0] = b'X';
        for (kind, bytes) in [
            (Kind::ServerMessage, &bytes),
            (Kind::Submission, &wrong_version),
            (Kind::Submission, &wrong_magic),
            (Kind::Submission, &bytes[..7].to_vec()),
        ] {
            let error = decode::<u64>(kind, bytes).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidEncoding);
        }
    }
}
