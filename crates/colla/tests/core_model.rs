use colla::{
    path, transform_pair, AttrChange, AttrPatch, AttrValue, Attrs, Change, ListChange, ListOp,
    MapChange, MapEntryChange, RichSpan, RichText, RichTextChange, RichTextOp, TextChange, TextOp,
    TieBreak, Value,
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

    let change = Change::map(
        MapChange::from_entries([
            (
                "title",
                MapEntryChange::Modify(Change::text(TextChange::new(vec![
                    TextOp::Retain(6),
                    TextOp::Insert(" 2".into()),
                ]))),
            ),
            (
                "tasks",
                MapEntryChange::Modify(Change::list(ListChange::new(vec![
                    ListOp::Retain(1),
                    ListOp::Modify(Change::map(
                        MapChange::from_entries([(
                            "done",
                            MapEntryChange::Modify(Change::replace(Value::bool(true))),
                        )])
                        .unwrap(),
                    )),
                    ListOp::Insert(vec![Value::map([("done", Value::bool(false))]).unwrap()]),
                ]))),
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
fn builder_is_sequential_and_transactional() {
    let base = Value::map([(
        "profile",
        Value::map([("name", Value::string("Old")), ("age", Value::int(30))]).unwrap(),
    )])
    .unwrap();
    let mut builder = base.change();
    builder
        .replace(
            &path!["profile"],
            Value::map([
                ("name", Value::string("Temporary")),
                ("age", Value::int(20)),
            ])
            .unwrap(),
        )
        .unwrap();
    builder
        .replace(&path!["profile", "name"], Value::string("Alice"))
        .unwrap();
    let before_error = builder.change().clone();
    assert!(builder.replace(&path!["missing"], Value::null()).is_err());
    assert_eq!(builder.change(), &before_error);
    let after = builder.build().apply_to(&base).unwrap();
    assert_eq!(
        after.get(&path!["profile", "name"]).unwrap(),
        &Value::string("Alice")
    );
}

#[test]
fn builder_map_set_and_list_edits_use_current_snapshot() {
    let base = Value::map([
        (
            "meta",
            Value::map([("status", Value::string("draft"))]).unwrap(),
        ),
        (
            "items",
            Value::list([Value::string("a"), Value::string("b")]),
        ),
    ])
    .unwrap();
    let mut builder = base.change();

    builder
        .map_set(&path!["meta"], "status", Value::string("draft"))
        .unwrap()
        .map_delete(&path!["meta"], "missing")
        .unwrap()
        .map_set(&path!["meta"], "status", Value::string("published"))
        .unwrap()
        .map_set(&path!["meta"], "owner", Value::string("team"))
        .unwrap()
        .list_insert(&path!["items"], 1, [Value::string("x")])
        .unwrap()
        .list_set(&path!["items"], 2, Value::string("b"))
        .unwrap()
        .list_delete(&path!["items"], 0, 1)
        .unwrap();

    let change = builder.build();
    let after = change.apply_to(&base).unwrap();
    assert_eq!(
        after,
        Value::map([
            (
                "meta",
                Value::map([
                    ("owner", Value::string("team")),
                    ("status", Value::string("published")),
                ])
                .unwrap(),
            ),
            (
                "items",
                Value::list([Value::string("x"), Value::string("b")]),
            ),
        ])
        .unwrap()
    );
}

#[test]
fn builder_container_errors_are_transactional() {
    let base = Value::map([("items", Value::list([Value::int(1)]))]).unwrap();
    let mut builder = base.change();
    let before = builder.change().clone();

    assert!(builder
        .list_insert(&path!["items"], 2, [Value::int(2)])
        .is_err());
    assert_eq!(builder.change(), &before);
    assert!(builder.list_delete(&path!["items"], 1, usize::MAX).is_err());
    assert_eq!(builder.change(), &before);
    assert!(builder
        .map_set(&path!["items"], "key", Value::null())
        .is_err());
    assert_eq!(builder.change(), &before);
}

#[test]
fn builder_map_list_bytes_match_javascript_golden() {
    let base = Value::map([
        (
            "meta",
            Value::map([("status", Value::string("draft"))]).unwrap(),
        ),
        (
            "items",
            Value::list([Value::string("a"), Value::string("b")]),
        ),
    ])
    .unwrap();
    let mut builder = base.change();
    builder
        .map_set(&path!["meta"], "status", Value::string("published"))
        .unwrap()
        .map_set(&path!["meta"], "owner", Value::string("team"))
        .unwrap()
        .list_insert(&path!["items"], 1, [Value::string("x")])
        .unwrap()
        .list_set(&path!["items"], 2, Value::string("B"))
        .unwrap()
        .list_delete(&path!["items"], 0, 1)
        .unwrap();

    assert_eq!(
        builder.build().encode(),
        [
            2, 2, 5, 105, 116, 101, 109, 115, 2, 3, 3, 1, 1, 5, 1, 120, 2, 1, 3, 1, 5, 1, 66, 4,
            109, 101, 116, 97, 2, 2, 2, 5, 111, 119, 110, 101, 114, 0, 5, 4, 116, 101, 97, 109, 6,
            115, 116, 97, 116, 117, 115, 2, 1, 5, 9, 112, 117, 98, 108, 105, 115, 104, 101, 100,
        ]
    );
}

#[test]
fn builder_text_replace_uses_scalar_positions_transactionally() {
    let base = Value::map([("title", Value::text("A😀B"))]).unwrap();
    let mut builder = base.change();
    builder
        .text_insert(&path!["title"], 2, "X")
        .unwrap()
        .text_delete(&path!["title"], 0, 1)
        .unwrap()
        .text_replace(&path!["title"], 1, 1, "Y")
        .unwrap();
    let change = builder.build();
    assert_eq!(
        change.encode(),
        [2, 1, 5, 116, 105, 116, 108, 101, 2, 4, 3, 2, 1, 0, 1, 1, 1, 89]
    );
    assert_eq!(
        change.apply_to(&base).unwrap().get(&path!["title"]),
        Some(&Value::text("😀YB"))
    );

    let mut invalid = base.change();
    let before = invalid.change().clone();
    assert!(invalid.text_delete(&path!["title"], 3, 1).is_err());
    assert_eq!(invalid.change(), &before);
    assert!(invalid.text_insert(&path!["title"], 4, "").is_err());
    assert_eq!(invalid.change(), &before);
}

#[test]
fn builder_rich_text_treats_embeds_as_atomic_units() {
    let base_attrs = Attrs::from_entries([
        ("bold", AttrValue::Bool(true)),
        ("count", AttrValue::Int(2)),
        ("opacity", AttrValue::float(0.5).unwrap()),
        ("label", AttrValue::string("base")),
    ])
    .unwrap();
    let base = Value::rich_text(RichText::new(vec![
        RichSpan::text("A😀", base_attrs.clone()),
        RichSpan::text("B", base_attrs),
        RichSpan::embed(
            Value::map([("id", Value::string("one"))]).unwrap(),
            Attrs::from_entries([
                ("kind", AttrValue::string("mention")),
                ("bold", AttrValue::Bool(true)),
            ])
            .unwrap(),
        ),
        RichSpan::text("C", Attrs::new()),
    ]));
    assert_eq!(base.as_rich_text().unwrap().spans().len(), 3);
    let value_golden = [
        7, 3, 0, 6, 65, 240, 159, 152, 128, 66, 4, 4, 98, 111, 108, 100, 1, 5, 99, 111, 117, 110,
        116, 2, 4, 5, 108, 97, 98, 101, 108, 4, 4, 98, 97, 115, 101, 7, 111, 112, 97, 99, 105, 116,
        121, 3, 0, 0, 0, 0, 0, 0, 224, 63, 1, 9, 1, 2, 105, 100, 5, 3, 111, 110, 101, 2, 4, 98,
        111, 108, 100, 1, 4, 107, 105, 110, 100, 4, 7, 109, 101, 110, 116, 105, 111, 110, 0, 1, 67,
        0,
    ];
    assert_eq!(base.encode(), value_golden);
    assert_eq!(Value::decode(&value_golden).unwrap(), base);

    let mut builder = base.change();
    builder
        .rich_text_insert_text(
            &path![],
            3,
            "X",
            Attrs::from_entries([("italic", AttrValue::Bool(true))]).unwrap(),
        )
        .unwrap()
        .rich_text_insert_embed(
            &path![],
            5,
            Value::map([("id", Value::string("two"))]).unwrap(),
            Attrs::from_entries([("kind", AttrValue::string("chip"))]).unwrap(),
        )
        .unwrap()
        .rich_text_delete(&path![], 2, 1)
        .unwrap()
        .rich_text_format(
            &path![],
            2,
            3,
            AttrPatch::from_entries([
                ("bold", AttrChange::Remove),
                ("color", AttrChange::Set(AttrValue::string("red"))),
            ])
            .unwrap(),
        )
        .unwrap();
    let change = builder.build();
    assert_eq!(
        change.encode(),
        [
            5, 5, 0, 2, 0, 1, 0, 1, 88, 2, 5, 99, 111, 108, 111, 114, 4, 3, 114, 101, 100, 6, 105,
            116, 97, 108, 105, 99, 1, 2, 1, 0, 1, 2, 4, 98, 111, 108, 100, 1, 5, 99, 111, 108, 111,
            114, 0, 4, 3, 114, 101, 100, 1, 1, 9, 1, 2, 105, 100, 5, 3, 116, 119, 111, 2, 5, 99,
            111, 108, 111, 114, 4, 3, 114, 101, 100, 4, 107, 105, 110, 100, 4, 4, 99, 104, 105,
            112,
        ]
    );
    let after = change.apply_to(&base).unwrap();
    let rich = after.as_rich_text().unwrap();
    assert_eq!(rich.to_plain_string(), "A😀XC");
    assert_eq!(
        rich.spans()
            .iter()
            .filter(|span| matches!(span.content, colla::RichInsert::Embed(_)))
            .count(),
        2
    );
}

#[test]
fn algebra_bytes_match_javascript_golden_across_all_change_kinds() {
    let base = Value::map([
        ("count", Value::int(5)),
        (
            "meta",
            Value::map([("status", Value::string("draft"))]).unwrap(),
        ),
        (
            "items",
            Value::list([Value::string("a"), Value::string("b")]),
        ),
        ("title", Value::text("ab")),
        (
            "rich",
            Value::rich_text(RichText::new(vec![RichSpan::text("ab", Attrs::new())])),
        ),
        ("replace", Value::string("old")),
    ])
    .unwrap();

    let mut first_builder = base.change();
    first_builder
        .int_add(&path!["count"], 2)
        .unwrap()
        .map_set(&path!["meta"], "owner", Value::string("team"))
        .unwrap()
        .list_insert(&path!["items"], 1, [Value::string("x")])
        .unwrap()
        .text_insert(&path!["title"], 1, "X")
        .unwrap()
        .rich_text_insert_text(
            &path!["rich"],
            1,
            "X",
            Attrs::from_entries([("bold", AttrValue::Bool(true))]).unwrap(),
        )
        .unwrap()
        .replace(&path!["replace"], Value::string("middle"))
        .unwrap();
    let first = first_builder.build();
    let middle = first.apply_to(&base).unwrap();

    let mut second_builder = middle.change();
    second_builder
        .int_add(&path!["count"], 3)
        .unwrap()
        .map_set(&path!["meta"], "status", Value::string("published"))
        .unwrap()
        .list_delete(&path!["items"], 0, 1)
        .unwrap()
        .text_delete(&path!["title"], 0, 1)
        .unwrap()
        .rich_text_format(
            &path!["rich"],
            0,
            2,
            AttrPatch::from_entries([("color", AttrChange::Set(AttrValue::string("red")))])
                .unwrap(),
        )
        .unwrap()
        .replace(&path!["replace"], Value::string("final"))
        .unwrap();
    let second = second_builder.build();
    let combined = first.compose(&second).unwrap();
    let combined_golden = [
        2, 6, 5, 99, 111, 117, 110, 116, 2, 6, 10, 5, 105, 116, 101, 109, 115, 2, 3, 2, 1, 1, 5, 1,
        120, 2, 1, 4, 109, 101, 116, 97, 2, 2, 2, 5, 111, 119, 110, 101, 114, 0, 5, 4, 116, 101,
        97, 109, 6, 115, 116, 97, 116, 117, 115, 2, 1, 5, 9, 112, 117, 98, 108, 105, 115, 104, 101,
        100, 7, 114, 101, 112, 108, 97, 99, 101, 2, 1, 5, 5, 102, 105, 110, 97, 108, 4, 114, 105,
        99, 104, 2, 5, 2, 0, 1, 1, 5, 99, 111, 108, 111, 114, 0, 4, 3, 114, 101, 100, 1, 0, 1, 88,
        2, 4, 98, 111, 108, 100, 1, 5, 99, 111, 108, 111, 114, 4, 3, 114, 101, 100, 5, 116, 105,
        116, 108, 101, 2, 4, 2, 1, 1, 88, 2, 1,
    ];
    assert_eq!(combined.encode(), combined_golden);
    assert_eq!(Change::decode(&combined_golden).unwrap(), combined);

    let inverse = combined.invert(&base).unwrap();
    assert_eq!(
        inverse.encode(),
        [
            2, 6, 5, 99, 111, 117, 110, 116, 2, 6, 9, 5, 105, 116, 101, 109, 115, 2, 3, 2, 1, 1, 5,
            1, 97, 2, 1, 4, 109, 101, 116, 97, 2, 2, 2, 5, 111, 119, 110, 101, 114, 1, 6, 115, 116,
            97, 116, 117, 115, 2, 1, 5, 5, 100, 114, 97, 102, 116, 7, 114, 101, 112, 108, 97, 99,
            101, 2, 1, 5, 3, 111, 108, 100, 4, 114, 105, 99, 104, 2, 5, 2, 0, 1, 1, 5, 99, 111,
            108, 111, 114, 1, 2, 1, 5, 116, 105, 116, 108, 101, 2, 4, 2, 1, 1, 97, 2, 1,
        ]
    );

    let mut right_builder = base.change();
    right_builder
        .int_add(&path!["count"], 4)
        .unwrap()
        .map_set(&path!["meta"], "reviewer", Value::string("qa"))
        .unwrap()
        .list_insert(&path!["items"], 1, [Value::string("y")])
        .unwrap()
        .text_insert(&path!["title"], 1, "Y")
        .unwrap()
        .rich_text_insert_text(
            &path!["rich"],
            1,
            "Y",
            Attrs::from_entries([("italic", AttrValue::Bool(true))]).unwrap(),
        )
        .unwrap()
        .replace(&path!["replace"], Value::string("right"))
        .unwrap();
    let right = right_builder.build();
    let (left_prime, right_prime) = transform_pair(&first, &right, TieBreak::LeftFirst).unwrap();
    assert_eq!(
        left_prime.encode(),
        [
            2, 6, 5, 99, 111, 117, 110, 116, 2, 6, 4, 5, 105, 116, 101, 109, 115, 2, 3, 2, 0, 1, 1,
            1, 5, 1, 120, 4, 109, 101, 116, 97, 2, 2, 1, 5, 111, 119, 110, 101, 114, 0, 5, 4, 116,
            101, 97, 109, 7, 114, 101, 112, 108, 97, 99, 101, 2, 1, 5, 6, 109, 105, 100, 100, 108,
            101, 4, 114, 105, 99, 104, 2, 5, 2, 0, 1, 0, 1, 0, 1, 88, 1, 4, 98, 111, 108, 100, 1,
            5, 116, 105, 116, 108, 101, 2, 4, 2, 0, 1, 1, 1, 88,
        ]
    );
    assert_eq!(
        right_prime.encode(),
        [
            2, 5, 5, 99, 111, 117, 110, 116, 2, 6, 8, 5, 105, 116, 101, 109, 115, 2, 3, 2, 0, 2, 1,
            1, 5, 1, 121, 4, 109, 101, 116, 97, 2, 2, 1, 8, 114, 101, 118, 105, 101, 119, 101, 114,
            0, 5, 2, 113, 97, 4, 114, 105, 99, 104, 2, 5, 2, 0, 2, 0, 1, 0, 1, 89, 1, 6, 105, 116,
            97, 108, 105, 99, 1, 5, 116, 105, 116, 108, 101, 2, 4, 2, 0, 2, 1, 1, 89,
        ]
    );
}

#[test]
fn rich_text_format_has_explicit_patch_semantics() {
    let attrs = Attrs::from_entries([("color", AttrValue::string("red"))]).unwrap();
    let base = Value::rich_text(RichText::new(vec![RichSpan::text("hi", attrs)]));
    let patch = AttrPatch::from_entries([
        ("color", AttrChange::Remove),
        ("bold", AttrChange::Set(AttrValue::Bool(true))),
    ])
    .unwrap();
    let change = Change::rich_text(RichTextChange::new(vec![RichTextOp::Retain {
        len: 2,
        attrs: patch,
    }]));
    let after = change.apply_to(&base).unwrap();
    let rich = after.as_rich_text().unwrap();
    assert_eq!(rich.spans().len(), 1);
    assert_eq!(
        rich.spans()[0].attrs.get("bold"),
        Some(&AttrValue::Bool(true))
    );
    assert_eq!(rich.spans()[0].attrs.get("color"), None);
}

#[test]
fn canonical_sequence_changes_merge_and_chop() {
    let change = TextChange::new(vec![
        TextOp::Retain(1),
        TextOp::Retain(2),
        TextOp::Insert("a".into()),
        TextOp::Insert("b".into()),
        TextOp::Retain(9),
    ]);
    assert_eq!(
        change.ops(),
        &[TextOp::Retain(3), TextOp::Insert("ab".into())]
    );
}
