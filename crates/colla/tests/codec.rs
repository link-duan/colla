use colla::{codec, Change, InputLimits, MapChange, MapEntryChange, TextChange, TextOp, Value};

#[test]
fn value_and_change_roundtrip() {
    let limits = InputLimits::default();
    let value = Value::map([
        ("i", Value::int(i64::MIN)),
        ("f", Value::float(1.5).unwrap()),
        ("s", Value::string("atomic")),
        ("t", Value::text("editable")),
    ])
    .unwrap();
    let bytes = value.encode();
    assert_eq!(Value::decode(&bytes).unwrap(), value);
    assert_eq!(codec::decode_value(&bytes, &limits).unwrap(), value);

    let change = Change::map(
        MapChange::from_entries([(
            "t",
            MapEntryChange::Modify(Change::text(TextChange::new(vec![
                TextOp::Retain(8),
                TextOp::Insert("!".into()),
            ]))),
        )])
        .unwrap(),
    );
    let bytes = change.encode();
    assert_eq!(Change::decode(&bytes).unwrap(), change);
    assert_eq!(codec::decode_change(&bytes, &limits).unwrap(), change);
}

#[test]
fn decoder_rejects_noncanonical_change() {
    // TextChange([Retain(1), Retain(2)]) -- adjacent retains must merge.
    let bytes = [4, 2, 0, 1, 0, 2];
    assert!(Change::decode(&bytes).is_err());
}

#[test]
fn decoder_rejects_adjacent_huge_lengths_without_panicking() {
    fn put_varint(mut value: u64, out: &mut Vec<u8>) {
        while value >= 0x80 {
            out.push((value as u8) | 0x80);
            value >>= 7;
        }
        out.push(value as u8);
    }

    // TextChange([Delete(usize::MAX), Delete(usize::MAX)]). Canonicalization
    // must reject adjacent deletes without first trying to add their lengths.
    let mut bytes = vec![4, 2, 2];
    put_varint(usize::MAX as u64, &mut bytes);
    bytes.push(2);
    put_varint(usize::MAX as u64, &mut bytes);

    assert!(Change::decode(&bytes).is_err());
}

#[test]
fn decoder_rejects_negative_zero() {
    let mut bytes = vec![4];
    bytes.extend_from_slice(&(-0.0f64).to_le_bytes());
    assert!(Value::decode(&bytes).is_err());
}

#[test]
fn input_limits_reject_large_logical_changes_only_when_decoding() {
    let change = Change::text(TextChange::new(vec![TextOp::Delete(9)]));
    let limits = InputLimits {
        max_sequence_len: 8,
        ..InputLimits::default()
    };

    assert!(Change::decode_with_limits(&change.encode(), &limits).is_err());
    assert_eq!(
        colla::transform_pair(&change, &Change::noop(), colla::TieBreak::LeftFirst).unwrap(),
        (change, Change::noop())
    );
}
