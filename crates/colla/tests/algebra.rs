use colla::{
    transform_pair, AttrChange, AttrPatch, AttrValue, Attrs, Change, ListChange, ListOp, MapChange,
    MapEntryChange, RichInsert, RichSpan, RichText, RichTextChange, RichTextOp, TextChange, TextOp,
    TieBreak, Value,
};

fn assert_tp1(base: &Value, a: &Change, b: &Change) {
    let (ap, bp) = transform_pair(a, b, TieBreak::LeftFirst).unwrap();
    let left = bp.apply_to(&a.apply_to(base).unwrap()).unwrap();
    let right = ap.apply_to(&b.apply_to(base).unwrap()).unwrap();
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
    let (ap, _) = transform_pair(&a, &b, TieBreak::LeftFirst).unwrap();
    let result = ap.apply_to(&b.apply_to(&base).unwrap()).unwrap();
    assert_eq!(result.as_map().unwrap().get("x"), Some(&Value::int(1)));
}

#[test]
fn compose_matches_sequential_apply() {
    let base = Value::text("hello");
    let a = Change::text(TextChange::new(vec![
        TextOp::Retain(5),
        TextOp::Insert("!".into()),
    ]));
    let b = Change::text(TextChange::new(vec![TextOp::Delete(1)]));
    let combined = a.compose(&b).unwrap();
    let sequential = b.apply_to(&a.apply_to(&base).unwrap()).unwrap();
    assert_eq!(combined.apply_to(&base).unwrap(), sequential);
}

#[test]
fn inverse_restores_base() {
    let base = Value::list([Value::int(1), Value::int(2), Value::int(3)]);
    let change = Change::list(ListChange::new(vec![
        ListOp::Retain(1),
        ListOp::Delete(1),
        ListOp::Insert(vec![Value::int(9)]),
    ]));
    let inverse = change.invert(&base).unwrap();
    let after = change.apply_to(&base).unwrap();
    assert_eq!(inverse.apply_to(&after).unwrap(), base);
}

#[test]
fn transform_keeps_huge_logical_sequences_compact() {
    let huge = Change::text(TextChange::new(vec![TextOp::Delete(usize::MAX)]));
    assert_eq!(
        transform_pair(&huge, &Change::noop(), TieBreak::LeftFirst).unwrap(),
        (huge, Change::noop())
    );
}

#[test]
fn text_compose_splits_unicode_insert_without_losing_boundaries() {
    let base = Value::text("ab");
    let first = Change::text(TextChange::new(vec![
        TextOp::Retain(1),
        TextOp::Insert("你🙂好".into()),
    ]));
    let second = Change::text(TextChange::new(vec![TextOp::Retain(2), TextOp::Delete(1)]));

    let combined = first.compose(&second).unwrap();
    let sequential = second.apply_to(&first.apply_to(&base).unwrap()).unwrap();

    assert_eq!(sequential, Value::text("a你好b"));
    assert_eq!(combined.apply_to(&base).unwrap(), sequential);
}

#[test]
fn list_compose_partially_consumes_insert_with_modify_and_delete() {
    let base = Value::list([Value::int(0)]);
    let first = Change::list(ListChange::new(vec![ListOp::Insert(vec![
        Value::int(1),
        Value::int(2),
        Value::int(3),
    ])]));
    let second = Change::list(ListChange::new(vec![
        ListOp::Retain(1),
        ListOp::Modify(Change::int_add(10)),
        ListOp::Delete(1),
    ]));

    let combined = first.compose(&second).unwrap();
    let sequential = second.apply_to(&first.apply_to(&base).unwrap()).unwrap();

    assert_eq!(
        sequential,
        Value::list([Value::int(1), Value::int(12), Value::int(0)])
    );
    assert_eq!(combined.apply_to(&base).unwrap(), sequential);
}

#[test]
fn rich_text_compose_batches_attrs_and_partially_consumes_insert() {
    let bold = Attrs::from_entries([("bold", AttrValue::Bool(true))]).unwrap();
    let red =
        AttrPatch::from_entries([("color", AttrChange::Set(AttrValue::string("red")))]).unwrap();
    let base = Value::rich_text(RichText::new(vec![RichSpan::text("z", Attrs::new())]));
    let first = Change::rich_text(RichTextChange::new(vec![RichTextOp::Insert {
        content: RichInsert::text("abc"),
        attrs: bold,
    }]));
    let second = Change::rich_text(RichTextChange::new(vec![
        RichTextOp::Retain { len: 1, attrs: red },
        RichTextOp::Delete(1),
    ]));

    let combined = first.compose(&second).unwrap();
    let sequential = second.apply_to(&first.apply_to(&base).unwrap()).unwrap();

    let rich = sequential.as_rich_text().unwrap();
    assert_eq!(rich.to_plain_string(), "acz");
    assert_eq!(
        rich.spans()[0].attrs.get("bold"),
        Some(&AttrValue::Bool(true))
    );
    assert_eq!(
        rich.spans()[0].attrs.get("color"),
        Some(&AttrValue::string("red"))
    );
    assert_eq!(combined.apply_to(&base).unwrap(), sequential);
}

#[test]
fn large_retain_compose_stays_compact() {
    let len = 900_000;
    let first = Change::text(TextChange::new(vec![
        TextOp::Retain(len),
        TextOp::Insert("x".into()),
    ]));
    let second = Change::text(TextChange::new(vec![
        TextOp::Retain(len),
        TextOp::Insert("y".into()),
    ]));

    let combined = first.compose(&second).unwrap();

    assert_eq!(
        combined,
        Change::text(TextChange::new(vec![
            TextOp::Retain(len),
            TextOp::Insert("yx".into()),
        ]))
    );
}
