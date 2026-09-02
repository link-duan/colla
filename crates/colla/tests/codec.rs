use colla::{codec, Change, CodecError, MapChange, MapEntryChange, TextChange, TextOp, Value};

#[test]
fn value_and_change_roundtrip() {
    let value = Value::map([
        ("i", Value::int(i64::MIN)),
        ("f", Value::float(1.5).unwrap()),
        ("s", Value::string("atomic")),
        ("t", Value::text("editable")),
    ])
    .unwrap();
    let bytes = value.encode();
    assert_eq!(Value::decode(&bytes).unwrap(), value);
    assert_eq!(codec::decode_value(&bytes).unwrap(), value);

    let change = Change::from(
        MapChange::from_entries([(
            "t",
            MapEntryChange::Modify(Change::from(
                TextChange::from_ops(vec![TextOp::Retain(8), TextOp::Insert("!".into())]).unwrap(),
            )),
        )])
        .unwrap(),
    );
    let bytes = change.encode();
    assert_eq!(Change::decode(&bytes).unwrap(), change);
    assert_eq!(codec::decode_change(&bytes).unwrap(), change);
}

#[test]
fn rich_text_roundtrips() {
    use colla::{Attrs, RichSpan, RichText};
    let value = Value::rich_text(
        RichText::from_spans(vec![
            RichSpan::text("hello ", Attrs::default()),
            RichSpan::embed(Value::int(1), Attrs::default()),
        ])
        .unwrap(),
    );
    let bytes = value.encode();
    assert_eq!(Value::decode(&bytes).unwrap(), value);
}

#[test]
fn negative_zero_normalizes_on_decode() {
    // cocodec carries f64 bits verbatim; FiniteF64::new normalizes -0.0 -> +0.0
    // on the way in, so decoding raw negative-zero bytes yields +0.0.
    let mut bytes = vec![3]; // ValueKind::Float tag
    bytes.extend_from_slice(&(-0.0f64).to_le_bytes());
    let value = Value::decode(&bytes).unwrap();
    assert_eq!(value, Value::float(0.0).unwrap());
}

#[test]
fn rejects_unsorted_map_keys() {
    // ValueKind::Map (tag 8): count 2, key "b" then key "a" -> not increasing.
    let bytes = [8, 2, 1, b'b', 0, 1, b'a', 0];
    assert!(matches!(
        Value::decode(&bytes),
        Err(CodecError::NonCanonical { .. })
    ));
}

#[test]
fn rejects_trailing_bytes() {
    // A complete Null value (tag 0) followed by an extra byte.
    assert!(matches!(
        Value::decode(&[0, 0xff]),
        Err(CodecError::TrailingBytes { .. })
    ));
}

#[test]
fn rejects_invalid_utf8() {
    // ValueKind::String (tag 4): length 1, byte 0xff.
    assert!(matches!(
        Value::decode(&[4, 1, 0xff]),
        Err(CodecError::InvalidUtf8 { .. })
    ));
}

#[test]
fn rejects_unknown_tag() {
    // No ValueKind variant claims tag 99.
    assert!(matches!(
        Value::decode(&[99]),
        Err(CodecError::UnknownTag { .. })
    ));
}

#[test]
fn decoder_rejects_huge_lengths_without_panicking() {
    fn put_varint(mut value: u64, out: &mut Vec<u8>) {
        while value >= 0x80 {
            out.push((value as u8) | 0x80);
            value >>= 7;
        }
        out.push(value as u8);
    }
    // ValueKind::List (tag 7) claiming a huge element count in a tiny input:
    // must be rejected up front, never pre-allocated.
    let mut bytes = vec![7];
    put_varint(u64::MAX, &mut bytes);
    assert!(Value::decode(&bytes).is_err());
}

#[test]
fn transform_still_works_without_decode_limits() {
    // Byte decoding no longer enforces InputLimits; large logical changes decode
    // fine and transform normally.
    let change = Change::from(TextChange::from_ops(vec![TextOp::Delete(9)]).unwrap());
    assert_eq!(Change::decode(&change.encode()).unwrap(), change);
    assert_eq!(
        colla::transform_pair(&change, &Change::noop(), colla::TieBreak::LeftFirst).unwrap(),
        (change, Change::noop())
    );
}
