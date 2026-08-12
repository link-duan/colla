use colla::{
    transform_pair, AttrChange, AttrPatch, AttrValue, Attrs, Change, IntChange, ListChange, ListOp,
    MapChange, MapEntryChange, RichContent, RichSpan, RichText, RichTextChange, RichTextOp,
    TextChange, TextOp, TieBreak, TransformError, Value,
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
    let a = Change::from(TextChange::from_ops(vec![TextOp::Retain(1), TextOp::Delete(2)]).unwrap());
    let b = Change::from(
        TextChange::from_ops(vec![TextOp::Retain(2), TextOp::Insert("X".into())]).unwrap(),
    );
    assert_tp1(&base, &a, &b);
}

#[test]
fn list_tp1_modify_delete() {
    let base = Value::list([Value::int(1), Value::int(2)]);
    let a = Change::from(ListChange::from_ops(vec![ListOp::Delete(1)]).unwrap());
    let b =
        Change::from(ListChange::from_ops(vec![ListOp::Modify(IntChange::Add(4).into())]).unwrap());
    assert_tp1(&base, &a, &b);
}

#[test]
fn map_insert_conflict_uses_tie_break() {
    let base = Value::map([] as [(&str, Value); 0]).unwrap();
    let a = Change::from(
        MapChange::from_entries([("x", MapEntryChange::Insert(Value::int(1)))]).unwrap(),
    );
    let b = Change::from(
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
    let a = Change::from(
        TextChange::from_ops(vec![TextOp::Retain(5), TextOp::Insert("!".into())]).unwrap(),
    );
    let b = Change::from(TextChange::from_ops(vec![TextOp::Delete(1)]).unwrap());
    let combined = a.compose(&b).unwrap();
    let sequential = b.apply_to(&a.apply_to(&base).unwrap()).unwrap();
    assert_eq!(combined.apply_to(&base).unwrap(), sequential);
}

#[test]
fn inverse_restores_base() {
    let base = Value::list([Value::int(1), Value::int(2), Value::int(3)]);
    let change = Change::from(
        ListChange::from_ops(vec![
            ListOp::Retain(1),
            ListOp::Delete(1),
            ListOp::Insert(vec![Value::int(9)]),
        ])
        .unwrap(),
    );
    let inverse = change.invert(&base).unwrap();
    let after = change.apply_to(&base).unwrap();
    assert_eq!(inverse.apply_to(&after).unwrap(), base);
}

#[test]
fn transform_keeps_huge_logical_sequences_compact() {
    let huge = Change::from(TextChange::from_ops(vec![TextOp::Delete(usize::MAX)]).unwrap());
    assert_eq!(
        transform_pair(&huge, &Change::noop(), TieBreak::LeftFirst).unwrap(),
        (huge, Change::noop())
    );
}

#[test]
fn text_compose_splits_unicode_insert_without_losing_boundaries() {
    let base = Value::text("ab");
    let first = Change::from(
        TextChange::from_ops(vec![TextOp::Retain(1), TextOp::Insert("你🙂好".into())]).unwrap(),
    );
    let second =
        Change::from(TextChange::from_ops(vec![TextOp::Retain(2), TextOp::Delete(1)]).unwrap());

    let combined = first.compose(&second).unwrap();
    let sequential = second.apply_to(&first.apply_to(&base).unwrap()).unwrap();

    assert_eq!(sequential, Value::text("a你好b"));
    assert_eq!(combined.apply_to(&base).unwrap(), sequential);
}

#[test]
fn list_compose_partially_consumes_insert_with_modify_and_delete() {
    let base = Value::list([Value::int(0)]);
    let first = Change::from(
        ListChange::from_ops(vec![ListOp::Insert(vec![
            Value::int(1),
            Value::int(2),
            Value::int(3),
        ])])
        .unwrap(),
    );
    let second = Change::from(
        ListChange::from_ops(vec![
            ListOp::Retain(1),
            ListOp::Modify(IntChange::Add(10).into()),
            ListOp::Delete(1),
        ])
        .unwrap(),
    );

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
    let base =
        Value::rich_text(RichText::from_spans(vec![RichSpan::text("z", Attrs::new())]).unwrap());
    let first = Change::from(
        RichTextChange::from_ops(vec![RichTextOp::Insert {
            content: RichContent::text("abc"),
            attrs: bold,
        }])
        .unwrap(),
    );
    let second = Change::from(
        RichTextChange::from_ops(vec![
            RichTextOp::Retain { len: 1, attrs: red },
            RichTextOp::Delete(1),
        ])
        .unwrap(),
    );

    let combined = first.compose(&second).unwrap();
    let sequential = second.apply_to(&first.apply_to(&base).unwrap()).unwrap();

    let rich = sequential.as_rich_text().unwrap();
    let first_span = rich.iter_spans().next().unwrap();
    assert_eq!(rich.to_plain_string(), "acz");
    assert_eq!(first_span.attrs().get("bold"), Some(&AttrValue::Bool(true)));
    assert_eq!(
        first_span.attrs().get("color"),
        Some(&AttrValue::string("red"))
    );
    assert_eq!(combined.apply_to(&base).unwrap(), sequential);
}

#[test]
fn rich_text_compose_repeatedly_splits_unicode_insert() {
    let base = Value::rich_text(RichText::default());
    let source_attrs = Attrs::from_entries([("source", AttrValue::Bool(true))]).unwrap();
    let first = Change::from(
        RichTextChange::from_ops(vec![RichTextOp::Insert {
            content: RichContent::text("A😀BC终"),
            attrs: source_attrs,
        }])
        .unwrap(),
    );
    let red =
        AttrPatch::from_entries([("color", AttrChange::Set(AttrValue::string("red")))]).unwrap();
    let bold = AttrPatch::from_entries([("bold", AttrChange::Set(AttrValue::Bool(true)))]).unwrap();
    let remove_source = AttrPatch::from_entries([("source", AttrChange::Remove)]).unwrap();
    let second = Change::from(
        RichTextChange::from_ops(vec![
            RichTextOp::Retain { len: 1, attrs: red },
            RichTextOp::Retain {
                len: 1,
                attrs: bold,
            },
            RichTextOp::Delete(1),
            RichTextOp::Retain {
                len: 1,
                attrs: remove_source,
            },
        ])
        .unwrap(),
    );

    let combined = first.compose(&second).unwrap();
    let sequential = second.apply_to(&first.apply_to(&base).unwrap()).unwrap();
    let actual = combined.apply_to(&base).unwrap();

    assert_eq!(actual, sequential);
    let rich = actual.as_rich_text().unwrap();
    assert_eq!(rich.to_plain_string(), "A😀C终");
    assert_eq!(rich.code_point_to_utf16(2), Ok(3));
    assert_eq!(rich.utf16_to_code_point(3), Ok(2));
}

#[test]
fn large_retain_compose_stays_compact() {
    let len = 900_000;
    let first = Change::from(
        TextChange::from_ops(vec![TextOp::Retain(len), TextOp::Insert("x".into())]).unwrap(),
    );
    let second = Change::from(
        TextChange::from_ops(vec![TextOp::Retain(len), TextOp::Insert("y".into())]).unwrap(),
    );

    let combined = first.compose(&second).unwrap();

    assert_eq!(
        combined,
        Change::from(
            TextChange::from_ops(vec![TextOp::Retain(len), TextOp::Insert("yx".into()),]).unwrap()
        )
    );
}

#[test]
fn rich_text_transform_reports_output_length_overflow() {
    let left =
        Change::from(RichTextChange::from_ops(vec![RichTextOp::Delete(usize::MAX)]).unwrap());
    let right = Change::from(
        RichTextChange::from_ops(vec![RichTextOp::Insert {
            content: RichContent::embed(Value::int(1)),
            attrs: Attrs::new(),
        }])
        .unwrap(),
    );

    assert_eq!(
        transform_pair(&left, &right, TieBreak::LeftFirst),
        Err(TransformError::LengthOverflow)
    );
}
