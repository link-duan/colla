use colla::{
    path, AttrChange, AttrPatch, AttrValue, Attrs, Change, ListChange, ListOp, MapChange,
    MapEntryChange, RichSpan, RichText, RichTextChange, RichTextOp, TextChange, TextOp,
    Utf16PositionError, Value,
};

#[test]
fn value_distinguishes_atomic_and_collaborative_strings() {
    assert_ne!(Value::string("hello"), Value::text("hello"));
    assert_ne!(Value::int(1), Value::float(1.0).unwrap());
    assert_eq!(Value::float(-0.0).unwrap(), Value::float(0.0).unwrap());
}

#[test]
fn recursive_change_updates_multiple_branches() {
    let base = Value::map([
        ("title", Value::text("Sprint")),
        (
            "tasks",
            Value::list([
                Value::map([("done", Value::bool(false))]).unwrap(),
                Value::map([("done", Value::bool(false))]).unwrap(),
            ]),
        ),
    ])
    .unwrap();

    let change = Change::from(
        MapChange::from_entries([
            (
                "title",
                MapEntryChange::Modify(
                    TextChange::from_ops(vec![TextOp::Retain(6), TextOp::Insert(" 2".into())])
                        .unwrap()
                        .into(),
                ),
            ),
            (
                "tasks",
                MapEntryChange::Modify(
                    ListChange::from_ops(vec![
                        ListOp::Retain(1),
                        ListOp::Modify(Change::from(
                            MapChange::from_entries([(
                                "done",
                                MapEntryChange::Modify(Change::replace(Value::bool(true))),
                            )])
                            .unwrap(),
                        )),
                        ListOp::Insert(vec![Value::map([("done", Value::bool(false))]).unwrap()]),
                    ])
                    .unwrap()
                    .into(),
                ),
            ),
        ])
        .unwrap(),
    );

    let after = change.apply_to(&base).unwrap();
    assert_eq!(
        after.get(&path!["title"]).unwrap(),
        &Value::text("Sprint 2")
    );
    assert_eq!(
        after.get(&path!["tasks", 1usize, "done"]).unwrap(),
        &Value::bool(true)
    );
    assert_eq!(
        after.get(&path!["tasks"]).unwrap().as_list().unwrap().len(),
        3
    );
}

#[test]
fn rich_text_format_has_explicit_patch_semantics() {
    let attrs = Attrs::from_entries([("color", AttrValue::string("red"))]).unwrap();
    let base = Value::rich_text(RichText::from_spans(vec![RichSpan::text("hi", attrs)]).unwrap());
    let patch = AttrPatch::from_entries([
        ("color", AttrChange::Remove),
        ("bold", AttrChange::Set(AttrValue::Bool(true))),
    ])
    .unwrap();
    let change: Change = RichTextChange::from_ops(vec![RichTextOp::Retain {
        len: 2,
        attrs: patch,
    }])
    .unwrap()
    .into();
    let after = change.apply_to(&base).unwrap();
    let rich = after.as_rich_text().unwrap();
    assert_eq!(rich.span_count(), 1);
    let span = rich.iter_spans().next().unwrap();
    assert_eq!(span.attrs().get("bold"), Some(&AttrValue::Bool(true)));
    assert_eq!(span.attrs().get("color"), None);
}

#[test]
fn rich_text_complete_span_apply_and_invert_round_trip() {
    let original = Attrs::from_entries([("bold", AttrValue::Bool(false))]).unwrap();
    let base =
        Value::rich_text(RichText::from_spans(vec![RichSpan::text("A😀B", original)]).unwrap());
    let change: Change = RichTextChange::from_ops(vec![RichTextOp::Retain {
        len: 3,
        attrs: AttrPatch::from_entries([("bold", AttrChange::Set(AttrValue::Bool(true)))]).unwrap(),
    }])
    .unwrap()
    .into();

    let inverse = change.invert(&base).unwrap();
    let after = change.apply_to(&base).unwrap();
    let rich = after.as_rich_text().unwrap();
    assert_eq!(rich.span_count(), 1);
    assert_eq!(rich.to_plain_string(), "A😀B");
    assert_eq!(
        rich.iter_spans().next().unwrap().attrs().get("bold"),
        Some(&AttrValue::Bool(true))
    );
    assert_eq!(inverse.apply_to(&after).unwrap(), base);
}

#[test]
fn rich_text_unicode_partial_span_apply_and_invert_round_trip() {
    let base = Value::rich_text(
        RichText::from_spans(vec![RichSpan::text("A😀终B", Attrs::new())]).unwrap(),
    );
    let red =
        AttrPatch::from_entries([("color", AttrChange::Set(AttrValue::string("red")))]).unwrap();
    let change: Change = RichTextChange::from_ops(vec![
        RichTextOp::Retain {
            len: 1,
            attrs: AttrPatch::new(),
        },
        RichTextOp::Retain { len: 2, attrs: red },
    ])
    .unwrap()
    .into();

    let inverse = change.invert(&base).unwrap();
    let after = change.apply_to(&base).unwrap();
    let rich = after.as_rich_text().unwrap();
    assert_eq!(rich.to_plain_string(), "A😀终B");
    assert_eq!(rich.span_count(), 3);
    assert_eq!(rich.iter_spans().nth(1).unwrap().content().len(), 2);
    assert_eq!(
        rich.iter_spans().nth(1).unwrap().attrs().get("color"),
        Some(&AttrValue::string("red"))
    );
    assert_eq!(rich.code_point_to_utf16(2), Ok(3));
    assert_eq!(rich.code_point_to_utf16(3), Ok(4));
    assert_eq!(inverse.apply_to(&after).unwrap(), base);
}

#[test]
fn rich_text_delete_crosses_text_embed_and_span_boundaries() {
    let first = Attrs::from_entries([("part", AttrValue::string("first"))]).unwrap();
    let middle = Attrs::from_entries([("part", AttrValue::string("middle"))]).unwrap();
    let last = Attrs::from_entries([("part", AttrValue::string("last"))]).unwrap();
    let base = Value::rich_text(
        RichText::from_spans(vec![
            RichSpan::text("ab", first),
            RichSpan::embed(Value::int(7), Attrs::new()),
            RichSpan::text("😀c", middle),
            RichSpan::text("de", last),
        ])
        .unwrap(),
    );
    let change: Change = RichTextChange::from_ops(vec![
        RichTextOp::Retain {
            len: 1,
            attrs: AttrPatch::new(),
        },
        RichTextOp::Delete(5),
    ])
    .unwrap()
    .into();

    let inverse = change.invert(&base).unwrap();
    let after = change.apply_to(&base).unwrap();
    let rich = after.as_rich_text().unwrap();
    assert_eq!(rich.to_plain_string(), "ae");
    assert_eq!(rich.span_count(), 2);
    assert_eq!(inverse.apply_to(&after).unwrap(), base);
}

#[test]
fn rich_text_caches_length_and_locates_unicode_and_embeds() {
    let bold = Attrs::from_entries([("bold", AttrValue::Bool(true))]).unwrap();
    let rich = RichText::from_spans(vec![
        RichSpan::text("A😀", bold.clone()),
        RichSpan::text("B", bold),
        RichSpan::embed(Value::int(7), Attrs::new()),
        RichSpan::text("终", Attrs::new()),
    ])
    .unwrap();

    assert_eq!(rich.len(), 5);
    assert_eq!(rich.span_count(), 3);
    assert_eq!(rich.locate_span(0), Some((0, 0)));
    assert_eq!(rich.locate_span(2), Some((0, 2)));
    assert_eq!(rich.locate_span(3), Some((1, 0)));
    assert_eq!(rich.locate_span(4), Some((2, 0)));
    assert_eq!(rich.locate_span(5), None);

    assert_eq!(rich.code_point_to_utf16(0), Ok(0));
    assert_eq!(rich.code_point_to_utf16(1), Ok(1));
    assert_eq!(rich.code_point_to_utf16(2), Ok(3));
    assert_eq!(rich.code_point_to_utf16(3), Ok(4));
    assert_eq!(rich.code_point_to_utf16(4), Ok(5));
    assert_eq!(rich.code_point_to_utf16(5), Ok(6));

    assert_eq!(rich.utf16_to_code_point(0), Ok(0));
    assert_eq!(rich.utf16_to_code_point(1), Ok(1));
    assert_eq!(rich.utf16_to_code_point(3), Ok(2));
    assert_eq!(rich.utf16_to_code_point(4), Ok(3));
    assert_eq!(rich.utf16_to_code_point(5), Ok(4));
    assert_eq!(rich.utf16_to_code_point(6), Ok(5));
    assert_eq!(
        rich.utf16_to_code_point(2),
        Err(Utf16PositionError::InvalidUtf16Boundary { position: 2 })
    );
    assert_eq!(
        rich.code_point_to_utf16(6),
        Err(Utf16PositionError::CodePointOutOfBounds {
            position: 6,
            len: 5,
        })
    );
    assert_eq!(
        rich.utf16_to_code_point(7),
        Err(Utf16PositionError::Utf16OutOfBounds {
            position: 7,
            len: 6,
        })
    );
}

#[test]
fn rich_text_coordinate_conversion_handles_many_spans_and_long_text() {
    let marked = Attrs::from_entries([("marked", AttrValue::Bool(true))]).unwrap();
    let spans = (0..10_000)
        .map(|index| {
            RichSpan::text(
                "😀",
                if index % 2 == 0 {
                    Attrs::new()
                } else {
                    marked.clone()
                },
            )
        })
        .collect();
    let many = RichText::from_spans(spans).unwrap();

    assert_eq!(many.span_count(), 10_000);
    assert_eq!(many.code_point_to_utf16(9_999), Ok(19_998));
    assert_eq!(many.utf16_to_code_point(19_998), Ok(9_999));
    assert_eq!(
        many.utf16_to_code_point(19_999),
        Err(Utf16PositionError::InvalidUtf16Boundary { position: 19_999 })
    );

    let text = format!("{}终", "a".repeat(100_000));
    let long = RichText::from_spans(vec![RichSpan::text(text, Attrs::new())]).unwrap();
    assert_eq!(long.code_point_to_utf16(100_000), Ok(100_000));
    assert_eq!(long.utf16_to_code_point(100_001), Ok(100_001));
}

#[test]
fn rich_text_change_checked_constructor_rejects_length_overflow() {
    let result = RichTextChange::from_ops(vec![
        RichTextOp::Retain {
            len: usize::MAX,
            attrs: AttrPatch::new(),
        },
        RichTextOp::Retain {
            len: 1,
            attrs: AttrPatch::new(),
        },
    ]);

    assert_eq!(result, Err(colla::ValueError::LengthOverflow));
}

#[test]
fn canonical_sequence_changes_merge_and_chop() {
    let change = TextChange::from_ops(vec![
        TextOp::Retain(1),
        TextOp::Retain(2),
        TextOp::Insert("a".into()),
        TextOp::Insert("b".into()),
        TextOp::Retain(9),
    ])
    .unwrap();
    assert_eq!(
        change.ops(),
        &[TextOp::Retain(3), TextOp::Insert("ab".into())]
    );
}
