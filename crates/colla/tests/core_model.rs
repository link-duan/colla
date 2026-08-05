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
