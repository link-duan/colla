//! Bounded, strictly canonical decoding of Value trees.

use super::{Attrs, Body, RichSpan, Value};
use cocodec::Decode;
use std::collections::BTreeMap;

// Count ownership depth once per Value, rather than once per implementation
// wrapper. Containers are bounded before allocation and map ordering is strict.
impl Decode for Value {
    fn decode<R: cocodec::Read>(
        d: &mut cocodec::Decoder<R>,
    ) -> std::result::Result<Self, cocodec::Error> {
        d.nested(|d| {
            let offset = d.offset();
            let invalid = |reason| cocodec::Error::NonCanonical { offset, reason };
            let mut body = match d.varint()? {
                0 => Body::Null,
                1 => Body::Bool(bool::decode(d)?),
                2 => Body::Int(i64::decode(d)?),
                3 => Body::Float(f64::decode(d)?),
                4 => Body::String(String::decode(d)?),
                5 => Body::Text(String::decode(d)?),
                6 => {
                    let length = d.varint()?;
                    if length > 1_000_000 {
                        return Err(invalid("rich span limit exceeded"));
                    }
                    let mut spans = Vec::new();
                    for _ in 0..length {
                        spans.push(match d.varint()? {
                            0 => RichSpan::Text {
                                text: String::decode(d)?,
                                attrs: Attrs::decode(d)?,
                            },
                            1 => RichSpan::Embed {
                                value: <Self as Decode>::decode(d)?,
                                attrs: Attrs::decode(d)?,
                            },
                            _ => return Err(invalid("invalid rich span tag")),
                        });
                    }
                    Body::RichText(spans)
                }
                7 => {
                    let length = d.varint()?;
                    if length > 1_000_000 {
                        return Err(invalid("list limit exceeded"));
                    }
                    let mut values = Vec::new();
                    for _ in 0..length {
                        values.push(<Self as Decode>::decode(d)?);
                    }
                    Body::List(values)
                }
                8 => {
                    let length = d.varint()?;
                    if length > 1_000_000 {
                        return Err(invalid("map limit exceeded"));
                    }
                    let mut values = BTreeMap::new();
                    for _ in 0..length {
                        let key = String::decode(d)?;
                        if values
                            .last_key_value()
                            .is_some_and(|(previous, _)| previous >= &key)
                        {
                            return Err(invalid("map keys must be unique and sorted"));
                        }
                        values.insert(key, <Self as Decode>::decode(d)?);
                    }
                    Body::Map(values)
                }
                _ => return Err(invalid("invalid Value tag")),
            };
            if super::canonicalize(&mut body).map_err(|_| invalid("invalid Value content"))? {
                return Err(invalid("noncanonical Value content"));
            }
            let value = Self::trusted(body);
            value
                .check_limits()
                .map_err(|_| invalid("value depth or node limit exceeded"))?;
            Ok(value)
        })
    }
}
