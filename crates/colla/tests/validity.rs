use colla::{
    Authority, Change, ErrorCode, Operation, RichSpan, Segment, ServerMessage, SessionCheckpoint,
    Submission, SyncSession, Value,
};

const MARKER: f64 = 1.5;

fn key(name: &str) -> Segment {
    Segment::Key(name.into())
}
/// Replaces every encoded `MARKER` float with a noncanonical `-0.0`.
fn negative_zero(mut bytes: Vec<u8>) -> Vec<u8> {
    let (from, to) = (MARKER.to_le_bytes(), (-0.0f64).to_le_bytes());
    let at = bytes.windows(8).position(|w| w == from).unwrap();
    bytes[at..at + 8].copy_from_slice(&to);
    bytes
}
fn assert_invalid<T: std::fmt::Debug>(result: colla::Result<T>, reason: &str) {
    let error = result.unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidEncoding);
    assert!(format!("{error:?}").contains(reason), "{error:?}");
}
fn nested_marker() -> Value {
    let leaf = Value::list(vec![Value::float(MARKER).unwrap()]).unwrap();
    Value::map([("a".into(), leaf)]).unwrap()
}

#[test]
fn value_decode_rejects_nested_noncanonical_float() {
    let bytes = nested_marker().encode();
    assert!(Value::decode(&bytes).is_ok());
    assert_invalid(Value::decode(&negative_zero(bytes)), "noncanonical Value");
}

#[test]
fn value_decode_rejects_noncanonical_rich_text() {
    let spans = vec![
        RichSpan::Text {
            text: "Q".into(),
            attrs: Default::default(),
        },
        RichSpan::Embed {
            value: Value::int(0),
            attrs: Default::default(),
        },
    ];
    let value = Value::list(vec![Value::rich_text(spans).unwrap()]).unwrap();
    let mut bytes = value.encode();
    assert!(Value::decode(&bytes).is_ok());
    // Shrinking the text to "" leaves an empty span that normalization removes.
    let at = bytes.windows(2).position(|w| w == [1, b'Q']).unwrap();
    bytes.splice(at..at + 2, [0]);
    assert_invalid(Value::decode(&bytes), "noncanonical Value");
}

#[test]
fn value_decode_rejects_depth_over_limit() {
    let deepest = (1..100).fold(Value::int(0), |value, _| Value::list(vec![value]).unwrap());
    let mut bytes = deepest.encode();
    assert!(Value::decode(&bytes).is_ok());
    // Wrap the payload, after the 8-byte envelope, in one more single-item List.
    bytes.splice(8..8, [7, 1]);
    assert_eq!(
        Value::decode(&bytes).unwrap_err().code,
        ErrorCode::InvalidEncoding
    );
}

#[test]
fn change_decode_rejects_noop_and_invalid_values() {
    let add = |delta| {
        Change::new([Operation::Add {
            path: vec![key("n")],
            delta,
        }])
        .unwrap()
        .encode()
    };
    let (five, six) = (add(5), add(6));
    let at = (0..five.len()).find(|&i| five[i] != six[i]).unwrap();
    let mut zero = five;
    zero[at] = 0;
    assert_invalid(Change::decode(&zero), "noncanonical change");

    let insert = Change::new([Operation::Insert {
        path: vec![key("x")],
        value: nested_marker(),
    }])
    .unwrap()
    .encode();
    assert_invalid(Change::decode(&negative_zero(insert)), "noncanonical Value");
}

#[test]
fn change_rejects_oversized_path_keys() {
    let path = vec![key(&"k".repeat(16 * 1024 * 1024 + 1))];
    let error = Change::new([Operation::Insert {
        path,
        value: Value::int(0),
    }])
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::LimitExceeded);
}

#[test]
fn protocol_decoders_reject_embedded_invalid_values() {
    let authority = Authority::create("doc", Value::map([]).unwrap()).unwrap();
    let session = SyncSession::create("a", authority.snapshot()).unwrap();
    session
        .document()
        .edit(|tx| tx.set(&[key("x")], nested_marker()))
        .unwrap();
    let submission = session.outbound().unwrap().unwrap();
    let checkpoint = session.checkpoint().unwrap();
    let (_, message) = authority.accept(&submission).unwrap();
    assert!(matches!(message, ServerMessage::Commit(_)));

    assert_invalid(
        Submission::decode(&negative_zero(submission.encode())),
        "noncanonical Value",
    );
    assert_invalid(
        ServerMessage::decode(&negative_zero(message.encode())),
        "noncanonical Value",
    );
    assert_invalid(
        SessionCheckpoint::decode(&negative_zero(checkpoint.encode())),
        "noncanonical Value",
    );
}
