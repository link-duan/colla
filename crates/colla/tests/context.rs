use colla::{codec, transform_pair, Change, Context, Limits, TextChange, TextOp, TieBreak, Value};

fn insert_at_one(value: &str) -> Change {
    Change::text(TextChange::new(vec![
        TextOp::Retain(1),
        TextOp::Insert(value.into()),
        TextOp::Retain(1),
    ]))
}

#[test]
fn context_owns_default_or_explicit_limits() {
    let default = Context::default();
    assert_eq!(default.limits(), &Limits::default());

    let limits = Limits {
        max_string_bytes: 32,
        ..Limits::default()
    };
    let expected = limits.clone();
    let context = Context::new(limits);
    assert_eq!(context.limits(), &expected);
}

#[test]
fn context_operations_match_existing_apis_and_canonical_bytes() {
    let context = Context::default();
    let limits = context.limits();
    let base = Value::text("ab");
    let first = insert_at_one("x");
    let after_first = first.apply_to(&base, limits).unwrap();
    let second = Change::text(TextChange::new(vec![
        TextOp::Retain(2),
        TextOp::Insert("y".into()),
        TextOp::Retain(1),
    ]));

    let applied = context.apply(&base, &first).unwrap();
    assert_eq!(applied, after_first);
    assert_eq!(
        codec::encode_value(&applied),
        codec::encode_value(&after_first)
    );

    let composed = context.compose(&first, &second).unwrap();
    let expected_composed = first.compose(&second, limits).unwrap();
    assert_eq!(composed, expected_composed);
    assert_eq!(
        codec::encode_change(&composed),
        codec::encode_change(&expected_composed)
    );

    let right = insert_at_one("z");
    assert_eq!(
        context
            .transform_pair(&first, &right, TieBreak::LeftFirst)
            .unwrap(),
        transform_pair(&first, &right, TieBreak::LeftFirst, limits).unwrap()
    );

    assert_eq!(
        context.invert(&first, &base).unwrap(),
        first.invert(&base, limits).unwrap()
    );

    let value_bytes = codec::encode_value(&base);
    assert_eq!(context.decode_value(&value_bytes).unwrap(), base);
    let change_bytes = codec::encode_change(&first);
    assert_eq!(context.decode_change(&change_bytes).unwrap(), first);
}

#[test]
fn context_preserves_existing_errors() {
    let limits = Limits {
        max_string_bytes: 1,
        ..Limits::default()
    };
    let context = Context::new(limits.clone());
    let oversized = Value::text("ab");
    let noop = Change::noop();

    assert_eq!(
        context.apply(&oversized, &noop).unwrap_err(),
        noop.apply_to(&oversized, &limits).unwrap_err()
    );

    let bytes = codec::encode_value(&oversized);
    assert_eq!(
        context.decode_value(&bytes).unwrap_err(),
        codec::decode_value(&bytes, &limits).unwrap_err()
    );

    let context = Context::default();
    let limits = context.limits();
    let text_change = insert_at_one("x");
    let int_change = Change::int_add(1);
    assert_eq!(
        context.compose(&text_change, &int_change).unwrap_err(),
        text_change.compose(&int_change, limits).unwrap_err()
    );
    assert_eq!(
        context
            .transform_pair(&text_change, &int_change, TieBreak::LeftFirst)
            .unwrap_err(),
        transform_pair(&text_change, &int_change, TieBreak::LeftFirst, limits).unwrap_err()
    );

    let text = Value::text("ab");
    assert_eq!(
        context.invert(&int_change, &text).unwrap_err(),
        int_change.invert(&text, limits).unwrap_err()
    );

    let noncanonical_change = [4, 2, 0, 1, 0, 2];
    assert_eq!(
        context.decode_change(&noncanonical_change).unwrap_err(),
        codec::decode_change(&noncanonical_change, limits).unwrap_err()
    );
}
