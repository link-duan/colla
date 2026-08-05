use colla::{codec, Change, Limits, MapChange, MapEntryChange, TextChange, TextOp, Value};

#[test]
fn value_and_change_roundtrip() {
    let limits = Limits::default();
    let value = Value::map([
        ("i", Value::int(i64::MIN)),
        ("f", Value::float(1.5).unwrap()),
        ("s", Value::string("atomic")),
        ("t", Value::text("editable")),
    ])
    .unwrap();
    let bytes = codec::encode_value(&value);
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
    let bytes = codec::encode_change(&change);
    assert_eq!(codec::decode_change(&bytes, &limits).unwrap(), change);
}

#[test]
fn decoder_rejects_noncanonical_change() {
    // TextChange([Retain(1), Retain(2)]) -- adjacent retains must merge.
    let bytes = [4, 2, 0, 1, 0, 2];
    assert!(codec::decode_change(&bytes, &Limits::default()).is_err());
}

#[test]
fn decoder_rejects_negative_zero() {
    let mut bytes = vec![4];
    bytes.extend_from_slice(&(-0.0f64).to_le_bytes());
    assert!(codec::decode_value(&bytes, &Limits::default()).is_err());
}
