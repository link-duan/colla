use colla::{
    apply, compose, invert, transform_pair, Change, ChangeBuilder, InputLimits, TextChange, TextOp,
    TieBreak, Value,
};

#[test]
fn package_functions_and_type_entrypoints_form_one_facade() {
    let base = Value::text("ab");
    let first = Change::text(TextChange::new(vec![
        TextOp::Retain(1),
        TextOp::Insert("x".into()),
    ]));
    let second = Change::text(TextChange::new(vec![
        TextOp::Retain(2),
        TextOp::Insert("y".into()),
    ]));

    let after_first = apply(&base, &first).unwrap();
    let combined = compose(&first, &second).unwrap();
    assert_eq!(apply(&base, &combined).unwrap(), Value::text("axyb"));
    assert_eq!(apply(&after_first, &second).unwrap(), Value::text("axyb"));

    let inverse = invert(&combined, &base).unwrap();
    assert_eq!(apply(&Value::text("axyb"), &inverse).unwrap(), base);

    let concurrent = Change::text(TextChange::new(vec![
        TextOp::Retain(1),
        TextOp::Insert("z".into()),
    ]));
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
fn builder_has_no_input_limit_policy() {
    let base = Value::text("a");
    let mut builder = base.change();
    builder.text_insert(&colla::Path::new(), 1, "bc").unwrap();
    let change = builder.build();
    assert_eq!(apply(&base, &change).unwrap(), Value::text("abc"));

    let direct = ChangeBuilder::new(&base);
    assert_eq!(direct.current(), &base);
}

#[test]
fn operation_results_are_independent_of_receiver_input_limits() {
    let base = Value::text("a");
    let change = Change::text(TextChange::new(vec![
        TextOp::Retain(1),
        TextOp::Insert("bc".into()),
    ]));
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
    let huge = Change::text(TextChange::new(vec![TextOp::Delete(usize::MAX)]));
    assert_eq!(
        transform_pair(&huge, &Change::noop(), TieBreak::LeftFirst).unwrap(),
        (huge, Change::noop())
    );
}
