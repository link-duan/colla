use colla::{
    path, AttrChange, AttrPatch, AttrValue, Attrs, Change, ListChange, ListOp, MapChange,
    MapEntryChange, RichSpan, RichText, RichTextChange, RichTextOp, TextChange, TextOp, Value,
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
