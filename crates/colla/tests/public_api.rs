use colla::{
    apply, compose, invert, transform_pair, ApplyError, Change, ChangeKind, CodecError,
    ComposeError, ErrorCode, InputLimits, IntChange, InvertError, ListChange, ListOp, MapChange,
    MapEntryChange, Path, RichTextChange, TextChange, TextOp, TieBreak, TransformError, Value,
    ValueError, ValueType,
};

#[test]
fn error_code_classification_is_stable_across_error_kinds() {
    let p = Path::new;

    assert_eq!(ValueError::NonFiniteFloat.code(), ErrorCode::InvalidValue);
    assert_eq!(
        ValueError::DuplicateKey("k".into()).code(),
        ErrorCode::InvalidValue
    );
    assert_eq!(ValueError::LengthOverflow.code(), ErrorCode::LimitExceeded);

    assert_eq!(
        ApplyError::TypeMismatch {
            path: p(),
            expected: ValueType::Int,
            actual: ValueType::Bool,
        }
        .code(),
        ErrorCode::TypeMismatch
    );
    assert_eq!(
        ApplyError::MissingKey {
            path: p(),
            key: "k".into(),
        }
        .code(),
        ErrorCode::MissingKey
    );
    assert_eq!(
        ApplyError::ExistingKey {
            path: p(),
            key: "k".into(),
        }
        .code(),
        ErrorCode::KeyAlreadyExists
    );
    assert_eq!(
        ApplyError::IndexOutOfBounds {
            path: p(),
            index: 1,
            len: 0,
        }
        .code(),
        ErrorCode::OutOfBounds
    );
    assert_eq!(
        ApplyError::SequenceOutOfBounds { path: p() }.code(),
        ErrorCode::OutOfBounds
    );
    assert_eq!(
        ApplyError::IntegerOverflow { path: p() }.code(),
        ErrorCode::IntegerOverflow
    );
    assert_eq!(
        ApplyError::SequenceLengthOverflow { path: p() }.code(),
        ErrorCode::LimitExceeded
    );

    assert_eq!(
        ComposeError::IncompatibleKinds {
            left: "a",
            right: "b",
        }
        .code(),
        ErrorCode::IncompatibleChange
    );
    assert_eq!(
        ComposeError::IncompatibleMapEntry("k".into()).code(),
        ErrorCode::IncompatibleChange
    );
    assert_eq!(
        ComposeError::LengthOverflow.code(),
        ErrorCode::LimitExceeded
    );
    assert_eq!(
        ComposeError::Apply(ApplyError::IntegerOverflow { path: p() }).code(),
        ErrorCode::IntegerOverflow
    );

    assert_eq!(
        TransformError::IncompatibleKinds {
            left: "a",
            right: "b",
        }
        .code(),
        ErrorCode::IncompatibleChange
    );
    assert_eq!(
        TransformError::IncompatibleMapEntry("k".into()).code(),
        ErrorCode::IncompatibleChange
    );
    assert_eq!(
        TransformError::LengthOverflow.code(),
        ErrorCode::LimitExceeded
    );

    assert_eq!(
        InvertError::Apply(ApplyError::MissingKey {
            path: p(),
            key: "k".into(),
        })
        .code(),
        ErrorCode::MissingKey
    );
    assert_eq!(InvertError::LengthOverflow.code(), ErrorCode::LimitExceeded);

    assert_eq!(
        CodecError::UnexpectedEof { offset: 0 }.code(),
        ErrorCode::InvalidEncoding
    );
    assert_eq!(
        CodecError::TrailingBytes { offset: 0 }.code(),
        ErrorCode::InvalidEncoding
    );
    assert_eq!(
        CodecError::LimitExceeded {
            name: "x",
            actual: 2,
            limit: 1,
        }
        .code(),
        ErrorCode::LimitExceeded
    );
    assert_eq!(
        CodecError::Value(ValueError::NonFiniteFloat).code(),
        ErrorCode::InvalidValue
    );

    assert_eq!(ErrorCode::InvalidEncoding.as_str(), "invalid_encoding");
    assert_eq!(ErrorCode::KeyAlreadyExists.as_str(), "key_already_exists");
    assert_eq!(ErrorCode::ALL.len(), 9);
    for code in ErrorCode::ALL {
        assert!(!code.as_str().is_empty());
    }
}

#[test]
fn package_functions_and_type_entrypoints_form_one_facade() {
    let base = Value::text("ab");
    let first = Change::from(
        TextChange::from_ops(vec![TextOp::Retain(1), TextOp::Insert("x".into())]).unwrap(),
    );
    let second = Change::from(
        TextChange::from_ops(vec![TextOp::Retain(2), TextOp::Insert("y".into())]).unwrap(),
    );

    let after_first = apply(&base, &first).unwrap();
    let combined = compose(&first, &second).unwrap();
    assert_eq!(apply(&base, &combined).unwrap(), Value::text("axyb"));
    assert_eq!(apply(&after_first, &second).unwrap(), Value::text("axyb"));

    let inverse = invert(&combined, &base).unwrap();
    assert_eq!(apply(&Value::text("axyb"), &inverse).unwrap(), base);

    let concurrent = Change::from(
        TextChange::from_ops(vec![TextOp::Retain(1), TextOp::Insert("z".into())]).unwrap(),
    );
    let (first_prime, concurrent_prime) =
        transform_pair(&first, &concurrent, TieBreak::LeftFirst).unwrap();
    assert_eq!(
        apply(&apply(&base, &first).unwrap(), &concurrent_prime).unwrap(),
        apply(&apply(&base, &concurrent).unwrap(), &first_prime).unwrap()
    );

    let bytes = combined.encode();
    assert_eq!(Change::decode(&bytes).unwrap(), combined);
    assert_eq!(
        Change::decode_with_limits(&bytes, &InputLimits::default()).unwrap(),
        combined
    );

    let value_bytes = after_first.encode();
    assert_eq!(Value::decode(&value_bytes).unwrap(), after_first);
    assert_eq!(
        Value::decode_with_limits(&value_bytes, &InputLimits::default()).unwrap(),
        after_first
    );
}

#[test]
fn typed_constructors_accept_iterators_and_convert_into_change() {
    let text: Change = TextChange::from_ops([TextOp::Retain(1), TextOp::Insert("x".into())])
        .unwrap()
        .into();
    assert!(matches!(text.kind(), ChangeKind::Text(_)));

    let list: Change = ListChange::from_ops(vec![ListOp::Insert(vec![Value::int(1)])])
        .unwrap()
        .into();
    assert!(matches!(list.kind(), ChangeKind::List(_)));

    let map: Change = MapChange::from_entries(
        [("key", Value::string("value"))]
            .into_iter()
            .map(|(key, value)| (key, MapEntryChange::Insert(value))),
    )
    .unwrap()
    .into();
    assert!(matches!(map.kind(), ChangeKind::Map(_)));

    let rich: Change = RichTextChange::from_ops(std::iter::empty()).unwrap().into();
    assert!(rich.is_noop());
    assert!(Change::from(TextChange::from_ops([]).unwrap()).is_noop());
    assert!(Change::from(ListChange::from_ops([]).unwrap()).is_noop());
    assert!(Change::from(MapChange::from_entries::<_, String>([]).unwrap()).is_noop());
    assert!(Change::from(IntChange::Add(0)).is_noop());
    assert!(matches!(
        Change::from(IntChange::Add(4)).kind(),
        ChangeKind::Int(_)
    ));
}

#[test]
fn operation_results_are_independent_of_receiver_input_limits() {
    let base = Value::text("a");
    let change = Change::from(
        TextChange::from_ops(vec![TextOp::Retain(1), TextOp::Insert("bc".into())]).unwrap(),
    );
    let result = apply(&base, &change).unwrap();
    let bytes = result.encode();
    let limits = InputLimits {
        max_string_bytes: 1,
        ..InputLimits::default()
    };

    assert!(Value::decode_with_limits(&bytes, &limits).is_err());
    assert_eq!(Value::decode(&bytes).unwrap(), result);
}

#[test]
fn huge_logical_changes_transform_without_an_operation_budget() {
    let huge = Change::from(TextChange::from_ops(vec![TextOp::Delete(usize::MAX)]).unwrap());
    assert_eq!(
        transform_pair(&huge, &Change::noop(), TieBreak::LeftFirst).unwrap(),
        (huge, Change::noop())
    );
}
