use colla::{
    transform_pair, Change, Limits, ListChange, ListOp, MapChange, MapEntryChange, TextChange,
    TextOp, TieBreak, Value,
};

fn assert_tp1(base: &Value, a: &Change, b: &Change) {
    let limits = Limits::default();
    let (ap, bp) = transform_pair(a, b, TieBreak::LeftFirst, &limits).unwrap();
    let left = bp
        .apply_to(&a.apply_to(base, &limits).unwrap(), &limits)
        .unwrap();
    let right = ap
        .apply_to(&b.apply_to(base, &limits).unwrap(), &limits)
        .unwrap();
    assert_eq!(left, right);
}

#[test]
fn text_tp1_insert_delete() {
    let base = Value::text("abcd");
    let a = Change::text(TextChange::new(vec![TextOp::Retain(1), TextOp::Delete(2)]));
    let b = Change::text(TextChange::new(vec![
        TextOp::Retain(2),
        TextOp::Insert("X".into()),
    ]));
    assert_tp1(&base, &a, &b);
}

#[test]
fn list_tp1_modify_delete() {
    let base = Value::list([Value::int(1), Value::int(2)]);
    let a = Change::list(ListChange::new(vec![ListOp::Delete(1)]));
    let b = Change::list(ListChange::new(vec![ListOp::Modify(Change::int_add(4))]));
    assert_tp1(&base, &a, &b);
}

#[test]
fn map_insert_conflict_uses_tie_break() {
    let base = Value::map([] as [(&str, Value); 0]).unwrap();
    let a = Change::map(
        MapChange::from_entries([("x", MapEntryChange::Insert(Value::int(1)))]).unwrap(),
    );
    let b = Change::map(
        MapChange::from_entries([("x", MapEntryChange::Insert(Value::int(2)))]).unwrap(),
    );
    assert_tp1(&base, &a, &b);
    let limits = Limits::default();
    let (ap, _) = transform_pair(&a, &b, TieBreak::LeftFirst, &limits).unwrap();
    let result = ap
        .apply_to(&b.apply_to(&base, &limits).unwrap(), &limits)
        .unwrap();
    assert_eq!(result.as_map().unwrap().get("x"), Some(&Value::int(1)));
}

#[test]
fn compose_matches_sequential_apply() {
    let limits = Limits::default();
    let base = Value::text("hello");
    let a = Change::text(TextChange::new(vec![
        TextOp::Retain(5),
        TextOp::Insert("!".into()),
    ]));
    let b = Change::text(TextChange::new(vec![TextOp::Delete(1)]));
    let combined = a.compose(&b, &limits).unwrap();
    let sequential = b
        .apply_to(&a.apply_to(&base, &limits).unwrap(), &limits)
        .unwrap();
    assert_eq!(combined.apply_to(&base, &limits).unwrap(), sequential);
}

#[test]
fn inverse_restores_base() {
    let limits = Limits::default();
    let base = Value::list([Value::int(1), Value::int(2), Value::int(3)]);
    let change = Change::list(ListChange::new(vec![
        ListOp::Retain(1),
        ListOp::Delete(1),
        ListOp::Insert(vec![Value::int(9)]),
    ]));
    let inverse = change.invert(&base, &limits).unwrap();
    let after = change.apply_to(&base, &limits).unwrap();
    assert_eq!(inverse.apply_to(&after, &limits).unwrap(), base);
}

#[test]
fn transform_checks_logical_sequence_limits_before_expansion() {
    let limits = Limits {
        max_sequence_len: 8,
        ..Limits::default()
    };
    let huge = Change::text(TextChange::new(vec![TextOp::Delete(usize::MAX)]));
    assert!(transform_pair(&huge, &Change::noop(), TieBreak::LeftFirst, &limits).is_err());
}
