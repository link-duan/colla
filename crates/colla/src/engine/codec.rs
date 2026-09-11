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

// A typed version-2 binary envelope around the Rust-owned canonical payload.
// No JavaScript code generates or parses protocol bytes.
pub(crate) fn encode<T: Encode>(kind: u8, value: &T) -> Vec<u8> {
    let mut bytes = Vec::from(MAGIC);
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.push(kind);
    value
        .encode(&mut bytes)
        .expect("writing to a Vec cannot fail");
    bytes
}
pub(crate) fn decode<T: Decode>(kind: u8, bytes: &[u8]) -> Result<T> {
    if bytes.len() < 8
        || &bytes[..5] != MAGIC
        || bytes[5..7] != 2u16.to_le_bytes()
        || bytes[7] != kind
    {
        return Err(Error::new(
            ErrorCode::InvalidEncoding,
            "invalid envelope magic, version, or kind",
        ));
    }
    cocodec::decode_from_slice(&bytes[8..])
        .map_err(|e| Error::new(ErrorCode::InvalidEncoding, e.to_string()))
}
